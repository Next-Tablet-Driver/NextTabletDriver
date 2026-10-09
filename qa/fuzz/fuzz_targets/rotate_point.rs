#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use next_tablet_driver::core::math::transform::rotate_point;

const COORDINATE_LIMIT: f32 = 1_000.0;
const ROTATION_LIMIT_DEGREES: f32 = 360.0;

/// Generates a finite f32 uniformly over a bounded domain.
fn bounded_f32(
    u: &mut Unstructured<'_>,
    min: f32,
    max: f32,
) -> arbitrary::Result<f32> {
    let value: u32 = u.arbitrary()?;
    let normalized = value as f32 / u32::MAX as f32;

    Ok(min + normalized * (max - min))
}

/// Compares two f32 values with a tolerance suitable for floating-point
/// geometric calculations.
fn approx_eq(a: f32, b: f32) -> bool {
    let tolerance = 1e-4_f32 * a.abs().max(b.abs()).max(1.0);

    (a - b).abs() <= tolerance
}

#[derive(Debug)]
struct RotateInput {
    x: f32,
    y: f32,
    center_x: f32,
    center_y: f32,
    rotation_degrees: f32,
}

impl<'a> Arbitrary<'a> for RotateInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            x: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            y: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            center_x: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            center_y: bounded_f32(u, -COORDINATE_LIMIT, COORDINATE_LIMIT)?,
            rotation_degrees: bounded_f32(
                u,
                -ROTATION_LIMIT_DEGREES,
                ROTATION_LIMIT_DEGREES,
            )?,
        })
    }
}

fuzz_target!(|input: RotateInput| {
    let RotateInput {
        x,
        y,
        center_x,
        center_y,
        rotation_degrees,
    } = input;

    let (rx, ry) = rotate_point(
        x,
        y,
        center_x,
        center_y,
        rotation_degrees,
    );

    // A valid finite input must produce finite output.
    assert!(
        rx.is_finite(),
        "rotate_point produced non-finite X: \
         input=({x}, {y}), center=({center_x}, {center_y}), \
         rotation={rotation_degrees}, result=({rx}, {ry})"
    );

    assert!(
        ry.is_finite(),
        "rotate_point produced non-finite Y: \
         input=({x}, {y}), center=({center_x}, {center_y}), \
         rotation={rotation_degrees}, result=({rx}, {ry})"
    );

    // Zero rotation is an exact identity in the implementation.
    if rotation_degrees == 0.0 {
        assert_eq!(rx.to_bits(), x.to_bits());
        assert_eq!(ry.to_bits(), y.to_bits());
    }

    // The rotation center must remain unchanged.
    if x == center_x && y == center_y {
        assert_eq!(rx.to_bits(), center_x.to_bits());
        assert_eq!(ry.to_bits(), center_y.to_bits());
    }

    // A rotation preserves the distance from the rotation center.
    let before_dx = x - center_x;
    let before_dy = y - center_y;

    let after_dx = rx - center_x;
    let after_dy = ry - center_y;

    let before_distance = before_dx.hypot(before_dy);
    let after_distance = after_dx.hypot(after_dy);

    assert!(
        approx_eq(before_distance, after_distance),
        "rotation did not preserve distance from center: \
         input=({x}, {y}), center=({center_x}, {center_y}), \
         rotation={rotation_degrees}, \
         before={before_distance}, after={after_distance}"
    );
});