//! Criterion benchmarks for the driver's pure coordinate transforms.
//!
//! These benchmarks isolate the mathematical primitives used by the input
//! pipeline. They intentionally contain no device I/O, synchronization,
//! allocation, plugin dispatch, or configuration lookup.
//!
//! The purpose is to:
//! - establish a stable performance baseline for the math layer;
//! - measure both common and branch-sensitive execution paths;
//! - detect regressions independently from the rest of the pipeline;
//! - provide enough granularity to identify which primitive changed.
//!
//! Run with:
//!
//!     cargo bench --bench transforms
//!
//! A benchmark result should be interpreted together with `pipeline.rs`.
//! The end-to-end pipeline remains the authoritative measurement for actual
//! driver performance.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use next_tablet_driver::core::math::transform::{
    apply_relative_delta, normalized_to_screen, physical_to_normalized, rotate_point,
};

/// Representative normalized tablet position.
const POINT_X: f32 = 0.73;
const POINT_Y: f32 = 0.31;

/// Representative normalized coordinate-space center.
const CENTER_X: f32 = 0.5;
const CENTER_Y: f32 = 0.5;

/// Representative physical tablet dimensions.
const TABLET_WIDTH_MM: f32 = 160.0;
const TABLET_HEIGHT_MM: f32 = 100.0;

/// Representative active-area center.
const AREA_X_MM: f32 = 80.0;
const AREA_Y_MM: f32 = 50.0;

/// Representative physical pen position.
const PEN_X_MM: f32 = 116.8;
const PEN_Y_MM: f32 = 31.0;

/// Representative previous pen position for relative movement.
const LAST_X_MM: f32 = 115.2;
const LAST_Y_MM: f32 = 30.4;

/// Representative display dimensions.
const SCREEN_X: f32 = 0.0;
const SCREEN_Y: f32 = 0.0;
const SCREEN_WIDTH: f32 = 2560.0;
const SCREEN_HEIGHT: f32 = 1440.0;

/// Representative relative-mode sensitivities.
const SENSITIVITY_X: f32 = 2.5;
const SENSITIVITY_Y: f32 = 2.0;

/// Benchmarks `rotate_point`, including its zero-rotation fast path.
fn bench_rotate_point(c: &mut Criterion) {
    let mut group = c.benchmark_group("transforms/rotate_point");

    group.bench_function("zero_rotation", |b| {
        b.iter_batched(
            || {
                (
                    black_box(POINT_X),
                    black_box(POINT_Y),
                    black_box(CENTER_X),
                    black_box(CENTER_Y),
                    black_box(0.0_f32),
                )
            },
            |(x, y, cx, cy, rotation)| {
                black_box(rotate_point(x, y, cx, cy, rotation));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("ninety_degrees", |b| {
        b.iter_batched(
            || {
                (
                    black_box(POINT_X),
                    black_box(POINT_Y),
                    black_box(CENTER_X),
                    black_box(CENTER_Y),
                    black_box(90.0_f32),
                )
            },
            |(x, y, cx, cy, rotation)| {
                black_box(rotate_point(x, y, cx, cy, rotation));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("arbitrary_rotation", |b| {
        b.iter_batched(
            || {
                (
                    black_box(POINT_X),
                    black_box(POINT_Y),
                    black_box(CENTER_X),
                    black_box(CENTER_Y),
                    black_box(17.5_f32),
                )
            },
            |(x, y, cx, cy, rotation)| {
                black_box(rotate_point(x, y, cx, cy, rotation));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmarks physical millimeter coordinates → normalized UV coordinates.
///
/// Both paths matter because `rotation == 0.0` avoids the rotation
/// calculation entirely in the production implementation.
fn bench_physical_to_normalized(c: &mut Criterion) {
    let mut group = c.benchmark_group("transforms/physical_to_normalized");

    group.bench_function("without_rotation", |b| {
        b.iter_batched(
            || {
                (
                    black_box(PEN_X_MM),
                    black_box(PEN_Y_MM),
                    black_box(AREA_X_MM),
                    black_box(AREA_Y_MM),
                    black_box(TABLET_WIDTH_MM),
                    black_box(TABLET_HEIGHT_MM),
                    black_box(0.0_f32),
                )
            },
            |(x, y, area_x, area_y, width, height, rotation)| {
                black_box(physical_to_normalized(
                    x, y, area_x, area_y, width, height, rotation,
                ));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("with_rotation", |b| {
        b.iter_batched(
            || {
                (
                    black_box(PEN_X_MM),
                    black_box(PEN_Y_MM),
                    black_box(AREA_X_MM),
                    black_box(AREA_Y_MM),
                    black_box(TABLET_WIDTH_MM),
                    black_box(TABLET_HEIGHT_MM),
                    black_box(30.0_f32),
                )
            },
            |(x, y, area_x, area_y, width, height, rotation)| {
                black_box(physical_to_normalized(
                    x, y, area_x, area_y, width, height, rotation,
                ));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmarks normalized UV coordinates → screen pixels.
///
/// The out-of-range case intentionally exercises the clamping path rather
/// than measuring only the most common in-range coordinates.
fn bench_normalized_to_screen(c: &mut Criterion) {
    let mut group = c.benchmark_group("transforms/normalized_to_screen");

    group.bench_function("in_range", |b| {
        b.iter_batched(
            || {
                (
                    black_box(POINT_X),
                    black_box(POINT_Y),
                    black_box(SCREEN_X),
                    black_box(SCREEN_Y),
                    black_box(SCREEN_WIDTH),
                    black_box(SCREEN_HEIGHT),
                )
            },
            |(u, v, x, y, width, height)| {
                black_box(normalized_to_screen(u, v, x, y, width, height));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("out_of_range_clamped", |b| {
        b.iter_batched(
            || {
                (
                    black_box(1.25_f32),
                    black_box(-0.25_f32),
                    black_box(SCREEN_X),
                    black_box(SCREEN_Y),
                    black_box(SCREEN_WIDTH),
                    black_box(SCREEN_HEIGHT),
                )
            },
            |(u, v, x, y, width, height)| {
                black_box(normalized_to_screen(u, v, x, y, width, height));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmarks relative tablet movement → pixel delta.
///
/// Both the no-rotation fast path and the rotated path are measured because
/// they exercise different amounts of floating-point work.
fn bench_apply_relative_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("transforms/apply_relative_delta");

    group.bench_function("without_rotation", |b| {
        b.iter_batched(
            || {
                (
                    black_box(PEN_X_MM),
                    black_box(PEN_Y_MM),
                    black_box(LAST_X_MM),
                    black_box(LAST_Y_MM),
                    black_box(0.0_f32),
                    black_box(SENSITIVITY_X),
                    black_box(SENSITIVITY_Y),
                )
            },
            |(x, y, last_x, last_y, rotation, sens_x, sens_y)| {
                black_box(apply_relative_delta(
                    x, y, last_x, last_y, rotation, sens_x, sens_y,
                ));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("with_rotation", |b| {
        b.iter_batched(
            || {
                (
                    black_box(PEN_X_MM),
                    black_box(PEN_Y_MM),
                    black_box(LAST_X_MM),
                    black_box(LAST_Y_MM),
                    black_box(30.0_f32),
                    black_box(SENSITIVITY_X),
                    black_box(SENSITIVITY_Y),
                )
            },
            |(x, y, last_x, last_y, rotation, sens_x, sens_y)| {
                black_box(apply_relative_delta(
                    x, y, last_x, last_y, rotation, sens_x, sens_y,
                ));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn transforms(c: &mut Criterion) {
    bench_rotate_point(c);
    bench_physical_to_normalized(c);
    bench_normalized_to_screen(c);
    bench_apply_relative_delta(c);
}

criterion_group!(benches, transforms);
criterion_main!(benches);
