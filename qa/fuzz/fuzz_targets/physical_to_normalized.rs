#![no_main]

mod common;

use arbitrary::{Arbitrary, Unstructured};
use common::{COORDINATE_LIMIT, ROTATION_LIMIT_DEGREES, bounded_f32};
use libfuzzer_sys::fuzz_target;
use next_tablet_driver::core::math::transform::physical_to_normalized;

#[derive(Debug)]
struct PhysicalInput {
    x_mm: f32,
    y_mm: f32,
    area_x: f32,
    area_y: f32,
    area_w: f32,
    area_h: f32,
    rotation: f32,
}

impl<'a> Arbitrary<'a> for PhysicalInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            x_mm: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            y_mm: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,

            area_x: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            area_y: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,

            // Active-area dimensions must be strictly positive.
            area_w: bounded_f32(u, 0.001, COORDINATE_LIMIT)?,
            area_h: bounded_f32(u, 0.001, COORDINATE_LIMIT)?,

            rotation: bounded_f32(u, -ROTATION_LIMIT_DEGREES, ROTATION_LIMIT_DEGREES)?,
        })
    }
}

fuzz_target!(|input: PhysicalInput| {
    let result = physical_to_normalized(
        input.x_mm,
        input.y_mm,
        input.area_x,
        input.area_y,
        input.area_w,
        input.area_h,
        input.rotation,
    );

    let (u, v) = result;

    assert!(
        u.is_finite(),
        "physical_to_normalized produced non-finite U: \
         input=({}, {}), area=({}, {}, {}, {}), rotation={}, result=({}, {})",
        input.x_mm,
        input.y_mm,
        input.area_x,
        input.area_y,
        input.area_w,
        input.area_h,
        input.rotation,
        u,
        v
    );

    assert!(
        v.is_finite(),
        "physical_to_normalized produced non-finite V: \
         input=({}, {}), area=({}, {}, {}, {}), rotation={}, result=({}, {})",
        input.x_mm,
        input.y_mm,
        input.area_x,
        input.area_y,
        input.area_w,
        input.area_h,
        input.rotation,
        u,
        v
    );

    // The center of the active area must map to the normalized center.
    let (u, v) = physical_to_normalized(
        input.area_x,
        input.area_y,
        input.area_x,
        input.area_y,
        input.area_w,
        input.area_h,
        0.0,
    );

    assert!((u - 0.5).abs() <= 1e-6);
    assert!((v - 0.5).abs() <= 1e-6);
});