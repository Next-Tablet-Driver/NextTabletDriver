//! Benchmarks for the engine's per-packet code path as the polling loop really runs it.
//!
//! `pipeline.rs` measures `Pipeline::process()` alone. This file measures everything the
//! polling thread does for one HID report **except** the HID read, the OS injection and the
//! shared-memory publication:
//!
//! ```text
//! driver.parse -> Pipeline::process -> inject (no-op sink) -> publish shared state
//!              -> stats -> UI channel
//! ```
//!
//! It calls the production function (`process_packet`, exposed to benchmarks through
//! `tablet_manager::bench_support`), so a change to that code shows up here directly.
//!
//! Two kinds of numbers are produced:
//!
//! 1. **Criterion** (mean + confidence interval) for each scenario.
//! 2. A **tail-latency table** (p50 / p99 / p99.9 / max, in ns per packet). The mean hides
//!    what hurts a pen: an occasional packet that is late because it waited on a lock.
//!
//! Scenarios, from best to worst case for lock contention:
//!
//! - `tray_hidden`: window hidden, nobody reads the shared state.
//! - `ui_60hz`: a "UI thread" snapshots the state and drains the channel at 60 Hz, like the
//!   real UI.
//! - `ui_stress`: a reader snapshots the state in a tight loop. Not realistic; it bounds the
//!   worst case.
//!
//! Run with:
//!
//!     cargo bench --bench hot_path
//!
//! Results are recorded in `qa/benches/RESULTS_hot_path.md`.

use criterion::{Criterion, Throughput, criterion_group};
use crossbeam_channel::bounded;
use next_tablet_driver::application::state::UiSnapshot;
use next_tablet_driver::core::config::models::{DriverMode, MappingConfig};
use next_tablet_driver::drivers::{NextTabletDriver, TabletData, TabletStatus};
use next_tablet_driver::engine::injector::InputSink;
use next_tablet_driver::engine::pipeline::Pipeline;
use next_tablet_driver::engine::state::SharedState;
use next_tablet_driver::engine::tablet_manager::bench_support::{
    PacketLoopState, process_packet_for_bench,
};
use next_tablet_driver::filters::FilterPipeline;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Packets timed together when building the tail-latency table. `Instant` has 100 ns
/// resolution on Windows, coarser than one packet, so a batch is timed and averaged.
const TAIL_BATCH: usize = 64;
/// Number of batches in the tail-latency table (about 1.3 M packets per scenario).
const TAIL_BATCHES: usize = 20_000;

/// Fixed driver: `parse` returns the same Contact report every time.
#[derive(Clone, Copy, Default)]
struct FixedDriver;

impl NextTabletDriver for FixedDriver {
    fn get_name(&self) -> &'static str {
        "Hot Path Bench Driver"
    }

    fn get_specs(&self) -> (f32, f32, f32) {
        (32_767.0, 32_767.0, 8_192.0)
    }

    fn get_physical_specs(&self) -> (f32, f32) {
        (160.0, 100.0)
    }

    fn get_vid_pid(&self) -> (u16, u16) {
        (0, 0)
    }

    fn parse(&self, raw: &[u8]) -> Option<TabletData> {
        // Derive coordinates from the report so the work cannot be constant-folded.
        let x = u32::from(*raw.first()?) << 7;
        let y = u32::from(*raw.get(1)?) << 7;
        Some(TabletData {
            status: TabletStatus::Contact,
            x,
            y,
            pressure: 4_096,
            tilt_x: 8,
            tilt_y: -4,
            is_connected: true,
            ..TabletData::default()
        })
    }
}

/// Input sink that does nothing observable, so no real input is injected.
#[derive(Default)]
struct NullSink {
    calls: u64,
}

impl InputSink for NullSink {
    fn set_proximity(&mut self, in_proximity: bool) {
        self.calls += u64::from(in_proximity);
    }

    fn move_absolute(&mut self, x: f32, y: f32, _u: f32, _v: f32, _p: i32, _tx: i32, _ty: i32) {
        self.calls += 1;
        black_box((x, y));
    }

    fn move_relative(&mut self, dx: f32, dy: f32) {
        self.calls += 1;
        black_box((dx, dy));
    }

    fn set_left_button(&mut self, is_down: bool) {
        self.calls += u64::from(is_down);
    }
}

#[derive(Clone, Copy, Debug)]
enum Scenario {
    TrayHidden,
    Ui60Hz,
    UiStress,
}

impl Scenario {
    const ALL: [Self; 3] = [Self::TrayHidden, Self::Ui60Hz, Self::UiStress];

    const fn name(self) -> &'static str {
        match self {
            Self::TrayHidden => "tray_hidden",
            Self::Ui60Hz => "ui_60hz",
            Self::UiStress => "ui_stress",
        }
    }
}

/// One engine thread's worth of state plus the optional "UI" reader thread.
struct Rig {
    shared: Arc<SharedState>,
    tx: crossbeam_channel::Sender<TabletData>,
    pipeline: Pipeline,
    filters: FilterPipeline,
    config: MappingConfig,
    sink: NullSink,
    loop_state: PacketLoopState,
    reader_stop: Arc<AtomicBool>,
    reader: Option<thread::JoinHandle<()>>,
    report: [u8; 8],
}

impl Rig {
    fn new(scenario: Scenario) -> Self {
        let shared = Arc::new(SharedState::new());
        let (tx, rx) = bounded::<TabletData>(256);
        let config = MappingConfig {
            mode: DriverMode::Absolute,
            ..MappingConfig::default()
        };
        let reader_stop = Arc::new(AtomicBool::new(false));

        let visible = !matches!(scenario, Scenario::TrayHidden);
        shared
            .lifecycle
            .is_visible
            .store(visible, Ordering::Relaxed);

        let reader = match scenario {
            Scenario::TrayHidden => None,
            Scenario::Ui60Hz | Scenario::UiStress => {
                let shared = Arc::clone(&shared);
                let stop = Arc::clone(&reader_stop);
                let stress = matches!(scenario, Scenario::UiStress);
                Some(thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        black_box(UiSnapshot::capture(&shared));
                        while rx.try_recv().is_ok() {}
                        if stress {
                            thread::yield_now();
                        } else {
                            thread::sleep(Duration::from_micros(16_667));
                        }
                    }
                }))
            }
        };

        Self {
            shared,
            tx,
            pipeline: Pipeline::new(),
            filters: FilterPipeline::new(),
            config,
            sink: NullSink::default(),
            loop_state: PacketLoopState::new(),
            reader_stop,
            reader,
            report: [0x81, 0x60, 0, 0, 0, 0, 0, 0],
        }
    }

    /// Processes one packet through the production per-packet code.
    #[inline]
    fn packet(&mut self) {
        // Vary the report a little so successive packets are not identical.
        self.report[0] = self.report[0].wrapping_add(1) | 0x40;
        process_packet_for_bench(
            black_box(&self.report),
            &FixedDriver,
            &self.shared,
            &self.tx,
            &mut self.pipeline,
            &mut self.sink,
            &mut self.filters,
            &self.config,
            &mut self.loop_state,
        );
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.reader_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
    }
}

fn bench_process_packet(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot_path/process_packet");
    group.throughput(Throughput::Elements(1));

    for scenario in Scenario::ALL {
        let mut rig = Rig::new(scenario);
        for _ in 0..10_000 {
            rig.packet(); // warm-up outside the timed region
        }
        group.bench_function(scenario.name(), |b| b.iter(|| rig.packet()));
    }
    group.finish();
}

/// Cost of the reader side, so a change that moves work from the engine thread to the UI
/// thread cannot hide.
fn bench_ui_snapshot(c: &mut Criterion) {
    let shared = Arc::new(SharedState::new());
    c.bench_function("hot_path/ui_snapshot_capture", |b| {
        b.iter(|| black_box(UiSnapshot::capture(&shared)));
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    targets = bench_process_packet, bench_ui_snapshot
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted.get(idx).copied().unwrap_or(0.0)
}

/// Prints per-packet latency percentiles for each scenario.
fn tail_report() {
    println!(
        "\nhot_path tail latency (ns per packet, {TAIL_BATCH}-packet batches x {TAIL_BATCHES})"
    );
    println!(
        "{:<14} {:>9} {:>9} {:>9} {:>9} {:>10}",
        "scenario", "p50", "p99", "p99.9", "max", "mean"
    );
    for scenario in Scenario::ALL {
        let mut rig = Rig::new(scenario);
        for _ in 0..50_000 {
            rig.packet();
        }

        let mut samples = Vec::with_capacity(TAIL_BATCHES);
        for _ in 0..TAIL_BATCHES {
            let start = Instant::now();
            for _ in 0..TAIL_BATCH {
                rig.packet();
            }
            samples.push(start.elapsed().as_nanos() as f64 / TAIL_BATCH as f64);
        }
        samples.sort_by(f64::total_cmp);
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        println!(
            "{:<14} {:>9.0} {:>9.0} {:>9.0} {:>9.0} {:>10.1}",
            scenario.name(),
            percentile(&samples, 0.50),
            percentile(&samples, 0.99),
            percentile(&samples, 0.999),
            samples.last().copied().unwrap_or(0.0),
            mean
        );
        black_box(rig.sink.calls);
    }
}

fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
    tail_report();
}
