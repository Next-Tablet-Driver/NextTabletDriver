//! The raw HID packet read/process loop that runs while this process owns
//! the tablet device.

use super::sdk_bridge::publish_shm_state;
use crate::core::config::models::{DriverMode, MappingConfig};
use crate::drivers::{TabletData, TabletStatus};
use crate::engine::injector::{Injector, InputSink};
use crate::engine::interop::shm::ShmWriter;
use crate::engine::pipeline::{Pipeline, ProcessedFrame};
use crate::engine::state::{LockRecoveryExt, SharedState, WriteRecoverExt};
use crate::filters::FilterPipeline;
use crossbeam_channel::Sender;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// The main packet reading loop of the engine thread.
///
/// Polls the raw HID device for byte reports and coordinates configuration reloading and packet
/// processing.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_polling_loop(
    device: &hidapi::HidDevice,
    driver: &dyn crate::drivers::NextTabletDriver,
    shared: &Arc<SharedState>,
    tablet_sender: &Sender<TabletData>,
    pipeline: &mut Pipeline,
    injector: &mut Injector,
    filters: &mut FilterPipeline,
    local_config: &mut MappingConfig,
    local_config_version: &mut u32,
    shm_writer: Option<&ShmWriter>,
) {
    let mut buf = [0u8; 64];
    let mut last_config_check = Instant::now();
    let mut last_stats_update = Instant::now();
    let mut last_packet_time: Option<(Instant, TabletStatus)> = None;

    loop {
        if shared.lifecycle.shutdown_requested.load(Ordering::Relaxed) {
            log::debug!(target: "TabletManager", "Shutdown requested, exiting polling loop");
            break;
        }

        if shared.config.reload_requested.load(Ordering::Relaxed) {
            log::debug!(target: "TabletManager", "Reload requested, exiting polling loop");
            break;
        }

        let read_start = Instant::now();
        match device.read_timeout(&mut buf, 500) {
            // Reduced from 1000 to 500 for faster shutdown check
            Ok(len) if len > 0 => {
                let read_duration = read_start.elapsed();
                if let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    if let Some(slice) = buf.get(..len) {
                        process_packet(
                            slice,
                            read_start,
                            read_duration,
                            driver,
                            shared,
                            tablet_sender,
                            pipeline,
                            injector,
                            filters,
                            local_config,
                            &mut last_stats_update,
                            &mut last_packet_time,
                            shm_writer,
                        );
                    }
                    maybe_reload_config(
                        shared,
                        filters,
                        local_config,
                        local_config_version,
                        &mut last_config_check,
                    );
                })) {
                    log::error!(target: "TabletManager", "Packet processing panicked: {e:?}");
                }
            }
            Ok(_) => {
                // Out of range event
                let out = TabletData {
                    status: TabletStatus::OutOfRange,
                    ..Default::default()
                };
                let frame = pipeline.process(&out, driver, local_config, filters, shared);
                inject_frame(injector, &out, local_config, &frame);
                *shared
                    .pipeline
                    .processed_frame
                    .write()
                    .unwrap_or_reset("processed_frame") = frame;
                *shared
                    .pipeline
                    .tablet_data
                    .write()
                    .unwrap_or_reset("tablet_data") = out.clone();
                if let Some(writer) = shm_writer {
                    publish_shm_state(writer, shared, &out, local_config, &frame);
                }
                if shared.lifecycle.is_visible.load(Ordering::Relaxed) {
                    let _ = tablet_sender.try_send(out);
                }

                // Still check for config even when out of range
                maybe_reload_config(
                    shared,
                    filters,
                    local_config,
                    local_config_version,
                    &mut last_config_check,
                );
            }
            Err(e) => {
                log::error!(target: "HID", "HID read error: {e}");
                return;
            }
        }
    }
}

/// Drives OS input injection from a processed frame.
///
/// Mirrors the branching that used to live inside `Pipeline::process()` itself:
/// disconnected pens release the button and drop proximity, non-positional
/// reports (aux/tool-ID) only drop proximity, and positional reports move the
/// cursor (absolute or relative, per the active driver mode) before syncing
/// the button state. Kept here rather than in the pipeline so that `Pipeline`
/// never touches the OS, which is required for the embedded SDK use case.
fn inject_frame<I: InputSink>(
    injector: &mut I,
    data: &TabletData,
    config: &MappingConfig,
    frame: &ProcessedFrame,
) {
    if !data.is_connected {
        injector.set_left_button(false);
        injector.set_proximity(false);
        return;
    }

    if !matches!(
        data.status,
        TabletStatus::Contact | TabletStatus::Hover | TabletStatus::Active
    ) {
        injector.set_proximity(false);
        return;
    }

    match config.mode {
        DriverMode::Absolute => {
            injector.move_absolute(
                frame.screen_x,
                frame.screen_y,
                frame.u,
                frame.v,
                frame.pressure,
                frame.tilt_x,
                frame.tilt_y,
            );
        }
        DriverMode::Relative => {
            injector.move_relative(frame.screen_x, frame.screen_y);
        }
    }

    injector.set_left_button(frame.is_down);
}

/// Minimum time between two refreshes of the driver statistics (~60 Hz).
const STATS_INTERVAL: Duration = Duration::from_millis(16);

/// Parses, processes, and submits a raw USB packet.
///
/// Evaluates parser and filter execution durations and reports performance lag spikes to
/// the logs, when parsing and processing time exceeds 5.0ms, or when the duration between
/// consecutive active reports exceeds 25.0ms.
///
/// Emits statistics updates and forwards output frames to the GUI thread.
#[allow(clippy::too_many_arguments)]
fn process_packet<I: InputSink>(
    raw: &[u8],
    read_start: Instant,
    read_duration: Duration,
    driver: &dyn crate::drivers::NextTabletDriver,
    shared: &Arc<SharedState>,
    tablet_sender: &Sender<TabletData>,
    pipeline: &mut Pipeline,
    injector: &mut I,
    filters: &mut FilterPipeline,
    local_config: &MappingConfig,
    last_stats_update: &mut Instant,
    last_packet_time: &mut Option<(Instant, TabletStatus)>,
    shm_writer: Option<&ShmWriter>,
) {
    // Reading the clock is not free (a QueryPerformanceCounter call on Windows, around
    // 20-30 ns) and this runs for every report, so it is done sparingly:
    // - one read at the start and one at the end give the total time (spike detection and
    //   the interval between packets) for every packet;
    // - the per-stage split (parse / inject) only feeds the statistics, which are refreshed
    //   about 60 times per second, so it is only measured for the packet that will refresh
    //   them (`detailed`).
    let packet_start = Instant::now();
    let detailed = packet_start.duration_since(*last_stats_update) > STATS_INTERVAL;
    if let Some(mut data) = driver.parse(raw) {
        let parse_end = detailed.then(Instant::now);
        let parse_duration = parse_end.map(|end| end.duration_since(packet_start));
        data.receive_time = Some(read_start);
        data.parser_time = parse_duration.unwrap_or_default();

        let frame = pipeline.process(&data, driver, local_config, filters, shared);

        let inject_start = detailed.then(Instant::now);
        inject_frame(injector, &data, local_config, &frame);
        let inject_duration = inject_start.map(|start| start.elapsed());

        *shared
            .pipeline
            .processed_frame
            .write()
            .unwrap_or_reset("processed_frame") = frame;
        *shared
            .pipeline
            .tablet_data
            .write()
            .unwrap_or_reset("tablet_data") = data.clone();
        if let Some(writer) = shm_writer {
            publish_shm_state(writer, shared, &data, local_config, &frame);
        }
        let now = Instant::now();
        let total_dur = now.duration_since(packet_start);
        if total_dur > Duration::from_millis(5) {
            log::warn!(
                target: "PerfSpike",
                "LAG SPIKE: Packet parsing & processing took {total_dur:.2?} (parsing: {parse_duration:.2?}, inject: {inject_duration:.2?}, HID read: {read_duration:.2?}; stage timings are only recorded for sampled packets)"
            );
        }

        if let Some((last_time, last_status)) = last_packet_time {
            let is_curr_active = !matches!(
                data.status,
                TabletStatus::Disconnected | TabletStatus::OutOfRange
            );
            let is_prev_active = !matches!(
                last_status,
                TabletStatus::Disconnected | TabletStatus::OutOfRange
            );
            if is_curr_active && is_prev_active {
                let interval = now.duration_since(*last_time);
                if interval > Duration::from_millis(25) {
                    log::warn!(
                        target: "PerfSpike",
                        "LAG SPIKE: Delay between active packets was {interval:.2?} (exceeded 25ms threshold)"
                    );
                }
            }
        }
        *last_packet_time = Some((now, data.status));

        shared.pipeline.packet_count.fetch_add(1, Ordering::Relaxed);

        // Update statistics (throttled to ~60Hz). `try_write`: if a reader holds the lock the
        // engine thread must not wait for it; the update is simply retried on the next packet.
        if let (true, Some(parse_duration), Some(inject_duration)) =
            (detailed, parse_duration, inject_duration)
            && let Ok(mut stats) = shared.pipeline.stats.try_write()
        {
            *last_stats_update = now;
            stats.total_packets = u64::from(shared.pipeline.packet_count.load(Ordering::Relaxed));

            let hr_ms = read_duration.as_secs_f32() * 1000.0;
            stats.hid_read_ms = hr_ms;
            stats.min_hid_read_ms = stats.min_hid_read_ms.min(hr_ms);
            stats.max_hid_read_ms = stats.max_hid_read_ms.max(hr_ms);
            stats.avg_hid_read_ms =
                (hr_ms - stats.avg_hid_read_ms).mul_add(0.05, stats.avg_hid_read_ms);

            let p_ms = parse_duration.as_secs_f32() * 1000.0;
            stats.parser_ms = p_ms;
            stats.min_parser_ms = stats.min_parser_ms.min(p_ms);
            stats.max_parser_ms = stats.max_parser_ms.max(p_ms);
            stats.avg_parser_ms = (p_ms - stats.avg_parser_ms).mul_add(0.05, stats.avg_parser_ms);

            let i_ms = inject_duration.as_secs_f32() * 1000.0;
            stats.inject_ms = i_ms;
            stats.min_inject_ms = stats.min_inject_ms.min(i_ms);
            stats.max_inject_ms = stats.max_inject_ms.max(i_ms);
            stats.avg_inject_ms = (i_ms - stats.avg_inject_ms).mul_add(0.05, stats.avg_inject_ms);
        }

        // Only send to the UI channel when the window is visible.
        // When hidden in the system tray, the UI thread is idle and
        // nobody consumes the channel - skipping prevents unbounded growth.
        if shared.lifecycle.is_visible.load(Ordering::Relaxed) {
            let _ = tablet_sender.try_send(data);
        }
    }
}

/// Loop-local state `process_packet` threads between calls, for benchmarks.
#[doc(hidden)]
pub struct PacketLoopState {
    last_stats_update: Instant,
    last_packet_time: Option<(Instant, TabletStatus)>,
}

impl PacketLoopState {
    /// Fresh state, as at the start of the polling loop.
    #[doc(hidden)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_stats_update: Instant::now(),
            last_packet_time: None,
        }
    }
}

impl Default for PacketLoopState {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the real per-packet code (parse, pipeline, injection through `injector`, state
/// publication, stats, UI channel) exactly as the polling loop does, minus the HID read and
/// the shared-memory publication. Not part of the public API: it exists so benchmarks measure
/// production code rather than a copy of it.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn process_packet_for_bench<I: InputSink>(
    raw: &[u8],
    driver: &dyn crate::drivers::NextTabletDriver,
    shared: &Arc<SharedState>,
    tablet_sender: &Sender<TabletData>,
    pipeline: &mut Pipeline,
    injector: &mut I,
    filters: &mut FilterPipeline,
    local_config: &MappingConfig,
    state: &mut PacketLoopState,
) {
    process_packet(
        raw,
        Instant::now(),
        Duration::ZERO,
        driver,
        shared,
        tablet_sender,
        pipeline,
        injector,
        filters,
        local_config,
        &mut state.last_stats_update,
        &mut state.last_packet_time,
        None,
    );
}

/// Checks for changed configuration versions and applies hot-reloading to the pipelines.
fn maybe_reload_config(
    shared: &Arc<SharedState>,
    filters: &mut FilterPipeline,
    local_config: &mut MappingConfig,
    local_config_version: &mut u32,
    last_check: &mut Instant,
) {
    if Instant::now().duration_since(*last_check) < Duration::from_millis(50) {
        return;
    }
    *last_check = Instant::now();

    let cv = shared.config.version.load(Ordering::Relaxed);
    if cv != *local_config_version {
        let config = shared.config.mapping.read().unwrap_or_log("config");
        *local_config = config.clone();
        drop(config);
        *local_config_version = cv;
        filters.update_config(local_config);
        log::info!(target: "Config", "Configuration reloaded to version {cv}");
        crate::settings::log_mapping_config(local_config, &format!("Reload v{cv}"));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::drivers::parsers::ReportParser;
    use crate::drivers::parsers::fallback::FallbackParser;

    /// Records calls instead of injecting real input, so running the tests never moves the
    /// developer's cursor.
    #[derive(Default)]
    struct RecordingSink {
        moves: usize,
    }
    impl InputSink for RecordingSink {
        fn set_proximity(&mut self, _in_proximity: bool) {}
        fn move_absolute(
            &mut self,
            _x: f32,
            _y: f32,
            _u: f32,
            _v: f32,
            _p: i32,
            _tx: i32,
            _ty: i32,
        ) {
            self.moves += 1;
        }
        fn move_relative(&mut self, _dx: f32, _dy: f32) {
            self.moves += 1;
        }
        fn set_left_button(&mut self, _is_down: bool) {}
    }

    struct MockDriver;
    impl crate::drivers::NextTabletDriver for MockDriver {
        fn get_name(&self) -> &'static str {
            "Mock"
        }
        fn get_specs(&self) -> (f32, f32, f32) {
            (1000.0, 1000.0, 8192.0)
        }
        fn get_physical_specs(&self) -> (f32, f32) {
            (100.0, 100.0)
        }
        fn get_vid_pid(&self) -> (u16, u16) {
            (0x1234, 0x5678)
        }
        fn parse(&self, data: &[u8]) -> Option<TabletData> {
            FallbackParser.parse(data)
        }
    }

    #[test]
    fn test_process_packet_updates_tablet_data_when_hidden_in_tray() {
        let shared = Arc::new(SharedState::new());
        // Window is minimized / hidden in system tray
        shared.lifecycle.is_visible.store(false, Ordering::Relaxed);

        let (tx, rx) = crossbeam_channel::bounded(16);
        let mut pipeline = Pipeline::new();
        let mut injector = RecordingSink::default();
        let mut filters = FilterPipeline::new();
        let local_config = MappingConfig {
            mode: DriverMode::Relative,
            ..Default::default()
        };

        let mut last_stats_update = Instant::now();
        let mut last_packet_time = None;

        let raw_report = [0x02, 0x01, 0x50, 0x00, 0x60, 0x00, 0x80, 0x00];
        let driver = MockDriver;

        process_packet(
            &raw_report,
            Instant::now(),
            Duration::from_millis(1),
            &driver,
            &shared,
            &tx,
            &mut pipeline,
            &mut injector,
            &mut filters,
            &local_config,
            &mut last_stats_update,
            &mut last_packet_time,
            None,
        );

        // Verify shared.pipeline.tablet_data is updated even when hidden in tray
        let data = shared.pipeline.tablet_data.read().unwrap().clone();
        assert_eq!(data.x, 80);
        assert_eq!(data.y, 96);
        assert_eq!(data.pressure, 128);

        // Channel should NOT have received the packet because is_visible is false
        assert!(rx.try_recv().is_err());

        // Now set is_visible to true and send another packet
        shared.lifecycle.is_visible.store(true, Ordering::Relaxed);
        let raw_report_2 = [0x02, 0x01, 0x70, 0x00, 0x80, 0x00, 0x90, 0x00];
        process_packet(
            &raw_report_2,
            Instant::now(),
            Duration::from_millis(1),
            &driver,
            &shared,
            &tx,
            &mut pipeline,
            &mut injector,
            &mut filters,
            &local_config,
            &mut last_stats_update,
            &mut last_packet_time,
            None,
        );

        let data_2 = shared.pipeline.tablet_data.read().unwrap().clone();
        assert_eq!(data_2.x, 112);
        assert_eq!(data_2.y, 128);
        assert_eq!(data_2.pressure, 144);

        // Channel should now receive the packet
        let received = rx
            .try_recv()
            .expect("UI channel should receive packet when visible");
        assert_eq!(received.x, 112);
        assert_eq!(received.y, 128);
        assert!(injector.moves >= 2, "every packet is injected");
    }

    /// The per-stage timings are only measured for the packet that refreshes the statistics.
    #[test]
    fn test_stats_are_refreshed_at_most_once_per_interval() {
        let shared = Arc::new(SharedState::new());
        shared.lifecycle.is_visible.store(false, Ordering::Relaxed);
        let (tx, _rx) = crossbeam_channel::bounded(16);
        let mut pipeline = Pipeline::new();
        let mut injector = RecordingSink::default();
        let mut filters = FilterPipeline::new();
        let config = MappingConfig::default();
        let driver = MockDriver;
        let report = [0x02, 0x01, 0x50, 0x00, 0x60, 0x00, 0x80, 0x00];

        // The stats were last refreshed long ago: this packet must refresh them.
        let mut last_stats_update = Instant::now()
            .checked_sub(Duration::from_millis(100))
            .expect("instant");
        let mut last_packet_time = None;
        let mut run = |last_stats_update: &mut Instant| {
            process_packet(
                &report,
                Instant::now(),
                Duration::from_micros(500),
                &driver,
                &shared,
                &tx,
                &mut pipeline,
                &mut injector,
                &mut filters,
                &config,
                last_stats_update,
                &mut last_packet_time,
                None,
            );
        };

        run(&mut last_stats_update);
        let after_first = *shared.pipeline.stats.read().unwrap();
        assert_eq!(after_first.total_packets, 1);
        assert!(after_first.hid_read_ms > 0.4 && after_first.hid_read_ms < 0.6);
        assert!(after_first.max_parser_ms >= 0.0 && after_first.max_inject_ms >= 0.0);

        // Right after, the interval has not elapsed: the statistics are left alone.
        run(&mut last_stats_update);
        run(&mut last_stats_update);
        let after_more = *shared.pipeline.stats.read().unwrap();
        assert_eq!(after_more.total_packets, 1);
        assert_eq!(shared.pipeline.packet_count.load(Ordering::Relaxed), 3);

        // Once the interval has elapsed again, they are refreshed.
        last_stats_update = Instant::now()
            .checked_sub(Duration::from_millis(100))
            .expect("instant");
        run(&mut last_stats_update);
        assert_eq!(shared.pipeline.stats.read().unwrap().total_packets, 4);
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;

        const REPORT: [u8; 8] = [0x02, 0x01, 0x50, 0x00, 0x60, 0x00, 0x80, 0x00];

        #[test]
        fn the_bench_entry_runs_the_real_per_packet_code() {
            let shared = Arc::new(SharedState::new());
            shared.lifecycle.is_visible.store(false, Ordering::Relaxed);
            let (tx, _rx) = crossbeam_channel::bounded(4);
            let mut pipeline = Pipeline::new();
            let mut sink = RecordingSink::default();
            let mut filters = FilterPipeline::new();
            let mut state = PacketLoopState::default();
            let config = MappingConfig::default();

            for _ in 0..3 {
                process_packet_for_bench(
                    &REPORT,
                    &MockDriver,
                    &shared,
                    &tx,
                    &mut pipeline,
                    &mut sink,
                    &mut filters,
                    &config,
                    &mut state,
                );
            }

            assert_eq!(shared.pipeline.packet_count.load(Ordering::Relaxed), 3);
            let data = shared.pipeline.tablet_data.read().unwrap().clone();
            assert_eq!((data.x, data.y, data.pressure), (80, 96, 128));
            assert!(sink.moves >= 3);
        }

        #[test]
        fn unparsable_reports_change_nothing() {
            let shared = Arc::new(SharedState::new());
            let (tx, rx) = crossbeam_channel::bounded(4);
            let mut pipeline = Pipeline::new();
            let mut sink = RecordingSink::default();
            let mut filters = FilterPipeline::new();
            let mut state = PacketLoopState::new();

            process_packet_for_bench(
                &[0x02],
                &MockDriver,
                &shared,
                &tx,
                &mut pipeline,
                &mut sink,
                &mut filters,
                &MappingConfig::default(),
                &mut state,
            );

            assert_eq!(shared.pipeline.packet_count.load(Ordering::Relaxed), 0);
            assert_eq!(sink.moves, 0);
            assert!(rx.try_recv().is_err());
        }

        fn long_ago() -> Instant {
            Instant::now()
                .checked_sub(Duration::from_secs(1))
                .expect("an instant one second in the past")
        }

        #[test]
        fn a_new_config_version_is_picked_up_by_the_polling_loop() {
            let shared = Arc::new(SharedState::new());
            shared.config.mapping.write().unwrap().active_area.w = 33.0;
            shared.config.version.store(7, Ordering::Relaxed);

            let mut filters = FilterPipeline::new();
            let mut local = MappingConfig::default();
            let mut version = 0;
            let mut last_check = long_ago();

            maybe_reload_config(
                &shared,
                &mut filters,
                &mut local,
                &mut version,
                &mut last_check,
            );

            assert_eq!(version, 7);
            assert_eq!(local.active_area.w, 33.0);
            assert!(last_check.elapsed() < Duration::from_millis(500));
        }

        #[test]
        fn the_config_is_only_checked_every_50_ms() {
            let shared = Arc::new(SharedState::new());
            shared.config.mapping.write().unwrap().active_area.w = 33.0;
            shared.config.version.store(7, Ordering::Relaxed);

            let mut filters = FilterPipeline::new();
            let mut local = MappingConfig::default();
            let original_width = local.active_area.w;
            let mut version = 0;
            let mut last_check = Instant::now();

            maybe_reload_config(
                &shared,
                &mut filters,
                &mut local,
                &mut version,
                &mut last_check,
            );

            assert_eq!(version, 0);
            assert_eq!(local.active_area.w, original_width);
        }

        #[test]
        fn an_unchanged_config_version_does_not_reload() {
            let shared = Arc::new(SharedState::new());
            shared.config.mapping.write().unwrap().active_area.w = 33.0;

            let mut filters = FilterPipeline::new();
            let mut local = MappingConfig::default();
            let original_width = local.active_area.w;
            let mut version = shared.config.version.load(Ordering::Relaxed);
            let mut last_check = long_ago();

            maybe_reload_config(
                &shared,
                &mut filters,
                &mut local,
                &mut version,
                &mut last_check,
            );

            assert_eq!(local.active_area.w, original_width);
        }
    }
}
