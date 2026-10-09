use arbitrary::{Arbitrary, Unstructured};

pub const COORDINATE_LIMIT: f32 = 1_000.0;
pub const ROTATION_LIMIT_DEGREES: f32 = 360.0;
pub const SCREEN_SIZE_LIMIT: f32 = 16_384.0;
pub const SENSITIVITY_LIMIT: f32 = 100.0;

pub fn bounded_f32(
    u: &mut Unstructured<'_>,
    min: f32,
    max: f32,
) -> arbitrary::Result<f32> {
    let value: u32 = u.arbitrary()?;

    let normalized = value as f32 / u32::MAX as f32;

    Ok(min + normalized * (max - min))
}