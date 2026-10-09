#![no_main]

mod common;

use arbitrary::{Arbitrary, Unstructured};
use common::{SCREEN_SIZE_LIMIT, bounded_f32};
use libfuzzer_sys::fuzz_target;
use next_tablet_driver::core::math::transform::normalized_to_screen;

#[derive(Debug)]
struct ScreenInput {
    u: f32,
    v: f32,

    target_x: f32,
    target_y: f32,

    target_w: f32,
    target_h: f32,
}

impl<'a> Arbitrary<'a> for ScreenInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            // Normalized coordinates are deliberately allowed outside
            // [0, 1] because the production function clamps them.
            u: bounded_f32(u, -2.0, 3.0)?,
            v: bounded_f32(u, -2.0, 3.0)?,

            target_x: bounded_f32(u, -SCREEN_SIZE_LIMIT, SCREEN_SIZE_LIMIT)?,
            target_y: bounded_f32(u, -SCREEN_SIZE_LIMIT, SCREEN_SIZE_LIMIT)?,

            // Screen dimensions must be positive.
            target_w: bounded_f32(u, 1.0, SCREEN_SIZE_LIMIT)?,
            target_h: bounded_f32(u, 1.0, SCREEN_SIZE_LIMIT)?,
        })
    }
}

fuzz_target!(|input: ScreenInput| {
    let (x, y) = normalized_to_screen(
        input.u,
        input.v,
        input.target_x,
        input.target_y,
        input.target_w,
        input.target_h,
    );

    assert!(
        x.is_finite(),
        "normalized_to_screen produced non-finite X: \
         input=({}, {}), target=({}, {}, {}, {}), result=({}, {})",
        input.u,
        input.v,
        input.target_x,
        input.target_y,
        input.target_w,
        input.target_h,
        x,
        y
    );

    assert!(
        y.is_finite(),
        "normalized_to_screen produced non-finite Y: \
         input=({}, {}), target=({}, {}, {}, {}), result=({}, {})",
        input.u,
        input.v,
        input.target_x,
        input.target_y,
        input.target_w,
        input.target_h,
        x,
        y
    );

    // Values below zero are clamped to the target origin.
    let (x, y) = normalized_to_screen(
        -1.0,
        -1.0,
        input.target_x,
        input.target_y,
        input.target_w,
        input.target_h,
    );

    assert_eq!(x.to_bits(), input.target_x.to_bits());
    assert_eq!(y.to_bits(), input.target_y.to_bits());

    // Values above one are clamped to the target's far edge.
    let (x, y) = normalized_to_screen(
        2.0,
        2.0,
        input.target_x,
        input.target_y,
        input.target_w,
        input.target_h,
    );

    assert_eq!(
        x.to_bits(),
        (input.target_x + input.target_w).to_bits()
    );

    assert_eq!(
        y.to_bits(),
        (input.target_y + input.target_h).to_bits()
    );
});