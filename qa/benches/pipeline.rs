//! End-to-end benchmarks for the input processing pipeline.
//!
//! This benchmark is intentionally focused on the production hot path:
//! `Pipeline::process()`. The goal is not to micro-optimize individual
//! arithmetic operations here; it is to establish a stable, representative
//! latency baseline for one tablet packet.
//!
//! Every benchmark keeps construction, configuration, cache warm-up, and other
//! setup work outside the timed region. Only the work performed for the packet
//! itself should contribute to the measured result.
//!
//! Run with:
//!
//!     cargo bench --bench pipeline
//!
//! For a narrower run:
//!
//!     cargo bench --bench pipeline -- pipeline/absolute
//!
//! See `benches/README.md` for the benchmark policy and interpretation rules.

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use next_tablet_driver::core::config::models::{DriverMode, MappingConfig, PressureCurveType};
use next_tablet_driver::drivers::{NextTabletDriver, TabletData, TabletStatus};
use next_tablet_driver::engine::pipeline::Pipeline;
use next_tablet_driver::engine::state::SharedState;
use next_tablet_driver::filters::FilterPipeline;
use std::hint::black_box;
use std::sync::Arc;

const MAX_X: f32 = 32_767.0;
const MAX_Y: f32 = 32_767.0;
const MAX_PRESSURE: f32 = 8_192.0;
const PHYSICAL_WIDTH_MM: f32 = 160.0;
const PHYSICAL_HEIGHT_MM: f32 = 100.0;

/// Deterministic driver implementation used to isolate the pipeline from
/// physical HID hardware.
///
/// Why: an end-to-end pipeline benchmark must exercise the real
/// `NextTabletDriver` abstraction without introducing USB, OS, or device
/// variability. Fixed values also make benchmark results reproducible.
#[derive(Clone, Copy, Debug, Default)]
struct BenchmarkDriver;

impl NextTabletDriver for BenchmarkDriver {
    fn get_name(&self) -> &'static str {
        "Benchmark Driver"
    }

    fn get_specs(&self) -> (f32, f32, f32) {
        (MAX_X, MAX_Y, MAX_PRESSURE)
    }

    fn get_physical_specs(&self) -> (f32, f32) {
        (PHYSICAL_WIDTH_MM, PHYSICAL_HEIGHT_MM)
    }

    fn get_vid_pid(&self) -> (u16, u16) {
        (0, 0)
    }

    fn parse(&self, _data: &[u8]) -> Option<TabletData> {
        None
    }
}

#[derive(Clone, Copy, Debug)]
enum FilterProfile {
    None,
}

impl FilterProfile {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "no_filters",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PressureProfile {
    Linear,
    Exponential,
    Custom,
}

impl PressureProfile {
    const fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Exponential => "exponential",
            Self::Custom => "custom_32_points",
        }
    }
}

/// Creates a representative packet in the normal Contact state.
///
/// Why: disconnected or ignored reports would measure an early return rather
/// than the actual processing path. Non-zero coordinates and pressure exercise
/// transformation, pressure processing, filtering and projection.
fn benchmark_packet(x: u32, y: u32, pressure: u16) -> TabletData {
    TabletData {
        status: TabletStatus::Contact,
        x,
        y,
        pressure,
        tilt_x: 8,
        tilt_y: -4,
        is_connected: true,
        ..TabletData::default()
    }
}

/// Builds the configuration used by one benchmark scenario.
///
/// Why: configuration construction is setup work, not packet-processing work.
/// Keeping it here prevents allocations and configuration mutations from
/// contaminating the latency measurement.
fn benchmark_config(mode: DriverMode, pressure: PressureProfile) -> MappingConfig {
    let mut config = MappingConfig {
        mode,
        ..MappingConfig::default()
    };

    config.active_area.x = 80.0;
    config.active_area.y = 50.0;
    config.active_area.w = 160.0;
    config.active_area.h = 100.0;
    config.active_area.rotation = 0.0;

    config.target_area.x = 0.0;
    config.target_area.y = 0.0;
    config.target_area.w = 1_920.0;
    config.target_area.h = 1_080.0;

    config.tip_threshold = 10;

    match pressure {
        PressureProfile::Linear => {
            config.pressure_curve.curve_type = PressureCurveType::Linear;
        }
        PressureProfile::Exponential => {
            config.pressure_curve.curve_type = PressureCurveType::Exponential;
            config.pressure_curve.exponent = 2.0;
        }
        PressureProfile::Custom => {
            config.pressure_curve.curve_type = PressureCurveType::Custom;
            config.pressure_curve.points = (0..=32)
                .map(|i| {
                    let x = i as f32 / 32.0;
                    (x, x * x)
                })
                .collect();
        }
    }

    config
}

/// Builds the filter chain for a benchmark scenario.
///
/// Why: filters are stateful and polymorphic. Constructing the chain inside
/// the timed closure would measure allocation and initialization instead of
/// per-packet filter processing.
const fn benchmark_filters(_profile: FilterProfile, _config: &mut MappingConfig) -> FilterPipeline {
    FilterPipeline::new()
}

struct BenchmarkState {
    pipeline: Pipeline,
    filters: FilterPipeline,
    data: TabletData,
    driver: BenchmarkDriver,
    config: MappingConfig,
    shared: Arc<SharedState>,
}

/// Creates one complete benchmark state.
///
/// Why: state is recreated for each Criterion batch so that one scenario cannot
/// leak mutable filter, projector or transformer state into another. This is
/// particularly important for relative mode and stateful filters.
fn benchmark_state(
    mode: DriverMode,
    filters: FilterProfile,
    pressure: PressureProfile,
) -> BenchmarkState {
    let mut config = benchmark_config(mode, pressure);
    let filters = benchmark_filters(filters, &mut config);

    BenchmarkState {
        pipeline: Pipeline::new(),
        filters,
        data: benchmark_packet(16_384, 12_288, 4_096),
        driver: BenchmarkDriver,
        config,
        shared: Arc::new(SharedState::new()),
    }
}

/// Measures one absolute-mode packet after persistent state has been warmed.
///
/// Why: this is the primary pipeline KPI. Absolute mode is the dominant tablet
/// workload, and the warm path represents the steady-state cost paid for each
/// incoming packet.
fn bench_absolute_warm(
    c: &mut Criterion,
    filter_profile: FilterProfile,
    pressure_profile: PressureProfile,
) {
    let name = format!(
        "pipeline/absolute/warm/{}/{}",
        filter_profile.name(),
        pressure_profile.name()
    );

    c.bench_function(&name, |b| {
        b.iter_batched(
            || {
                let mut state =
                    benchmark_state(DriverMode::Absolute, filter_profile, pressure_profile);

                // Warm the transformer's cached multipliers and stateful
                // processing before timing the representative packet.
                let _ = state.pipeline.process(
                    &state.data,
                    &state.driver,
                    &state.config,
                    &mut state.filters,
                    &state.shared,
                );

                state
            },
            |mut state| {
                black_box(state.pipeline.process(
                    &state.data,
                    &state.driver,
                    &state.config,
                    &mut state.filters,
                    &state.shared,
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

/// Measures the first packet through a freshly created absolute pipeline.
///
/// Why: this isolates cold initialization, especially the Transformer's
/// hardware-to-millimeter multiplier cache. It prevents a future optimization
/// from improving steady state while silently making first-use behavior worse.
fn bench_absolute_cold(c: &mut Criterion) {
    c.bench_function("pipeline/absolute/cold/no_filters/linear", |b| {
        b.iter_batched(
            || {
                benchmark_state(
                    DriverMode::Absolute,
                    FilterProfile::None,
                    PressureProfile::Linear,
                )
            },
            |mut state| {
                black_box(state.pipeline.process(
                    &state.data,
                    &state.driver,
                    &state.config,
                    &mut state.filters,
                    &state.shared,
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

/// Measures relative-mode steady-state processing.
///
/// Why: relative projection has different state and timing behavior from
/// absolute projection. `Projector::project_relative()` also calls
/// `Instant::now()`, so it must be benchmarked as production code rather than
/// replaced by a pure mathematical approximation.
fn bench_relative_warm(c: &mut Criterion) {
    c.bench_function("pipeline/relative/warm/no_filters/linear", |b| {
        b.iter_batched(
            || {
                let mut state = benchmark_state(
                    DriverMode::Relative,
                    FilterProfile::None,
                    PressureProfile::Linear,
                );

                // Establish a previous physical position so the measured
                // operation represents steady-state relative movement rather
                // than the special first-packet zero-delta path.
                let _ = state.pipeline.process(
                    &state.data,
                    &state.driver,
                    &state.config,
                    &mut state.filters,
                    &state.shared,
                );

                state.data.x = 16_500;
                state.data.y = 12_350;

                state
            },
            |mut state| {
                black_box(state.pipeline.process(
                    &state.data,
                    &state.driver,
                    &state.config,
                    &mut state.filters,
                    &state.shared,
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

/// Compares pressure-curve implementations while keeping the rest of the
/// processing path identical.
///
/// Why: pressure curves execute for every accepted packet. Exponential curves
/// use `powf`, while custom curves perform control-point traversal, so their
/// cost must remain visible in the end-to-end budget.
fn bench_pressure_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/absolute/warm/pressure");

    group.throughput(Throughput::Elements(1));

    for profile in [
        PressureProfile::Linear,
        PressureProfile::Exponential,
        PressureProfile::Custom,
    ] {
        group.bench_with_input(
            BenchmarkId::from_parameter(profile.name()),
            &profile,
            |b, &profile| {
                b.iter_batched(
                    || benchmark_state(DriverMode::Absolute, FilterProfile::None, profile),
                    |mut state| {
                        black_box(state.pipeline.process(
                            &state.data,
                            &state.driver,
                            &state.config,
                            &mut state.filters,
                            &state.shared,
                        ))
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Runs the complete benchmark matrix for the production absolute path.
///
/// Why: a single benchmark is easy to accidentally optimize for. A small
/// explicit matrix makes performance claims robust across the main filter
/// configurations while remaining practical for routine regression testing.
fn bench_absolute_matrix(c: &mut Criterion) {
    bench_absolute_warm(c, FilterProfile::None, PressureProfile::Linear);
}

criterion_group!(
    pipeline_benches,
    bench_absolute_cold,
    bench_absolute_matrix,
    bench_relative_warm,
    bench_pressure_curves
);

criterion_main!(pipeline_benches);
