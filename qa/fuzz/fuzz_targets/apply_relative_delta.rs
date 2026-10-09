#![no_main]

mod common;

use arbitrary::{Arbitrary, Unstructured};
use common::{COORDINATE_LIMIT, ROTATION_LIMIT_DEGREES, SENSITIVITY_LIMIT, bounded_f32};
use libfuzzer_sys::fuzz_target;
use next_tablet_driver::core::math::transform::apply_relative_delta;

#[derive(Debug)]
struct RelativeInput {
    x_mm: f32,
    y_mm: f32,

    last_x_mm: f32,
    last_y_mm: f32,

    rotation: f32,

    sens_x: f32,
    sens_y: f32,
}

impl<'a> Arbitrary<'a> for RelativeInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            x_mm: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            y_mm: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,

            last_x_mm: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            last_y_mm: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,

            rotation: bounded_f32(u, -ROTATION_LIMIT_DEGREES, ROTATION_LIMIT_DEGREES)?,

            sens_x: bounded_f32(u, 0.001, SENSITIVITY_LIMIT)?,
            sens_y: bounded_f32(u, 0.001, SENSITIVITY_LIMIT)?,
        })
    }
}

fuzz_target!(|input: RelativeInput| {
    let (dx, dy) = apply_relative_delta(
        input.x_mm,
        input.y_mm,
        input.last_x_mm,
        input.last_y_mm,
        input.rotation,
        input.sens_x,
        input.sens_y,
    );

    assert!(
        dx.is_finite(),
        "apply_relative_delta produced non-finite X: \
         input=({}, {}), last=({}, {}), rotation={}, \
         sensitivity=({}, {}), result=({}, {})",
        input.x_mm,
        input.y_mm,
        input.last_x_mm,
        input.last_y_mm,
        input.rotation,
        input.sens_x,
        input.sens_y,
        dx,
        dy
    );

    assert!(
        dy.is_finite(),
        "apply_relative_delta produced non-finite Y: \
         input=({}, {}), last=({}, {}), rotation={}, \
         sensitivity=({}, {}), result=({}, {})",
        input.x_mm,
        input.y_mm,
        input.last_x_mm,
        input.last_y_mm,
        input.rotation,
        input.sens_x,
        input.sens_y,
        dx,
        dy
    );

    // No movement must produce no delta.
    let (dx, dy) = apply_relative_delta(
        input.x_mm,
        input.y_mm,
        input.x_mm,
        input.y_mm,
        input.rotation,
        input.sens_x,
        input.sens_y,
    );

    assert_eq!(dx.to_bits(), 0.0f32.to_bits());
    assert_eq!(dy.to_bits(), 0.0f32.to_bits());

    // With zero rotation, sensitivity directly scales the physical delta.
    let (dx, dy) = apply_relative_delta(
        input.x_mm,
        input.y_mm,
        input.last_x_mm,
        input.last_y_mm,
        0.0,
        input.sens_x,
        input.sens_y,
    );

    let expected_dx = (input.x_mm - input.last_x_mm) * input.sens_x;
    let expected_dy = (input.y_mm - input.last_y_mm) * input.sens_y;

    assert_eq!(dx.to_bits(), expected_dx.to_bits());
    assert_eq!(dy.to_bits(), expected_dy.to_bits());
});