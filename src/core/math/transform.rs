//! # Mathematics and Transforms
//!
//! This module provides the mathematical functions required to map raw
//! physical tablet coordinates into operating system screen coordinates.
//! It handles aspects like rotation, active area cropping, absolute-to-relative
//! conversion, and respecting aspect ratios.

/// Applies 2D rotation around a center point in normalized space.
///
/// Rotation is applied clockwise in degrees. This function is typically used
/// to adjust for tablets that are physically rotated on the desk.
///
/// # Examples
///
/// ```
/// use next_tablet_driver::core::math::transform::rotate_point;
///
/// // No rotation should return the same point
/// let (x, y) = rotate_point(0.7, 0.2, 0.5, 0.5, 0.0);
/// assert!((x - 0.7).abs() < f32::EPSILON);
/// assert!((y - 0.2).abs() < f32::EPSILON);
///
/// // 90 degree rotation around center (0.5, 0.5)
/// // (0.5, 0.0) -> (0.0, 0.5)
/// let (x, y) = rotate_point(0.5, 0.0, 0.5, 0.5, 90.0);
/// assert!((x - 0.0).abs() < 1e-6);
/// assert!((y - 0.5).abs() < 1e-6);
/// ```
pub fn rotate_point(
    x: f32,
    y: f32,
    center_x: f32,
    center_y: f32,
    rotation_degrees: f32,
) -> (f32, f32) {
    if rotation_degrees == 0.0 {
        return (x, y);
    }
    // Convert to radians and invert sign for clockwise rotation in a Y-down coordinate system
    let rad = -rotation_degrees.to_radians();
    let (sin, cos) = rad.sin_cos();

    let cx = x - center_x;
    let cy = y - center_y;

    let rx = cx.mul_add(cos, -(cy * sin)) + center_x;
    let ry = cx.mul_add(sin, cy * cos) + center_y;

    (rx, ry)
}

/// Normalizes physical tablet coordinates (millimeters) into [0.0, 1.0] UV space.
///
/// This function maps the raw pen position from the tablet surface into a
/// unit-square representation relative to the user's defined active area.
///
/// # Arguments
/// * `x_mm`, `y_mm` - Raw physical position from the tablet (in millimeters).
/// * `area_x`, `area_y` - Center offset of the active area (in millimeters).
/// * `area_w`, `area_h` - Total width and height of the active area (in millimeters).
/// * `rotation` - Physical tablet rotation in degrees.
///
/// # Examples
///
/// ```
/// use next_tablet_driver::core::math::transform::physical_to_normalized;
///
/// // Map a point exactly in the middle of a 100x100 area centered at (50, 50)
/// let (u, v) = physical_to_normalized(50.0, 50.0, 50.0, 50.0, 100.0, 100.0, 0.0);
/// assert_eq!(u, 0.5);
/// assert_eq!(v, 0.5);
/// ```
pub fn physical_to_normalized(
    x_mm: f32,
    y_mm: f32,
    area_x: f32,
    area_y: f32,
    area_w: f32,
    area_h: f32,
    rotation: f32,
) -> (f32, f32) {
    let mut u = (x_mm - area_x) / area_w + 0.5;
    let mut v = (y_mm - area_y) / area_h + 0.5;

    if rotation != 0.0 {
        let (nu, nv) = rotate_point(u, v, 0.5, 0.5, rotation);
        u = nu;
        v = nv;
    }

    (u, v)
}

/// Projects normalized UV coordinates `[0.0, 1.0]` onto screen pixels.
///
/// Inputs are clamped to the `[0.0, 1.0]` range to ensure the cursor stays
/// within the bounds of the target screen area.
///
/// # Arguments
/// * `u`, `v` - Normalized percentage coordinates (0.0 to 1.0).
/// * `target_x`, `target_y` - Top-left origin of the destination screen area (pixels).
/// * `target_w`, `target_h` - Dimensions of the destination screen area (pixels).
///
/// # Examples
///
/// ```
/// use next_tablet_driver::core::math::transform::normalized_to_screen;
///
/// // Map the center of UV space to a 1920x1080 monitor starting at (0, 0)
/// let (x, y) = normalized_to_screen(0.5, 0.5, 0.0, 0.0, 1920.0, 1080.0);
/// assert_eq!(x, 960.0);
/// assert_eq!(y, 540.0);
/// ```
pub fn normalized_to_screen(
    u: f32,
    v: f32,
    target_x: f32,
    target_y: f32,
    target_w: f32,
    target_h: f32,
) -> (f32, f32) {
    let u_clamped = u.clamp(0.0, 1.0);
    let v_clamped = v.clamp(0.0, 1.0);

    let screen_x = target_x + u_clamped * target_w;
    let screen_y = target_y + v_clamped * target_h;

    (screen_x, screen_y)
}

/// Computes the pixel delta for relative (mouse-like) movement.
///
/// This mode is typically used for general desktop navigation where
/// the pen acts like a high-precision touchpad.
///
/// # Arguments
/// * `x_mm`, `y_mm` - Current physical location of the pen.
/// * `last_x_mm`, `last_y_mm` - Previous physical location of the pen.
/// * `rotation` - Movement vector rotation in degrees.
/// * `sens_x`, `sens_y` - Sensitivity multipliers (Pixels per Millimeter).
pub fn apply_relative_delta(
    x_mm: f32,
    y_mm: f32,
    last_x_mm: f32,
    last_y_mm: f32,
    rotation: f32,
    sens_x: f32,
    sens_y: f32,
) -> (f32, f32) {
    let mut dx_mm = x_mm - last_x_mm;
    let mut dy_mm = y_mm - last_y_mm;

    // Apply rotation to the movement vector
    if rotation != 0.0 {
        let rad = rotation.to_radians();
        let (sin, cos) = rad.sin_cos();
        let rx = dx_mm.mul_add(cos, -(dy_mm * sin));
        let ry = dx_mm.mul_add(sin, dy_mm * cos);
        dx_mm = rx;
        dy_mm = ry;
    }

    // Convert millimeter movement into pixel movement
    let dx_px = dx_mm * sens_x;
    let dy_px = dy_mm * sens_y;

    (dx_px, dy_px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotate_point_90_deg() {
        let center = (0.5, 0.5);
        // Top middle rotated 90 degrees around center should be Left middle
        let (x, y) = rotate_point(0.5, 0.0, center.0, center.1, 90.0);
        assert!((x - 0.0).abs() < 1e-6);
        assert!((y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_point_180_deg() {
        let center = (0.5, 0.5);
        // Top left rotated 180 degrees should be Bottom right
        let (x, y) = rotate_point(0.0, 0.0, center.0, center.1, 180.0);
        assert!((x - 1.0).abs() < 1e-6);
        assert!((y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalization_clamping_behavior() {
        // Test that physical_to_normalized does NOT clamp (it should allow over-hovering)
        let (u, v) = physical_to_normalized(150.0, 150.0, 50.0, 50.0, 100.0, 100.0, 0.0);
        assert!(u > 1.0);
        assert!(v > 1.0);
    }

    #[test]
    fn test_screen_projection_clamping() {
        // Test that normalized_to_screen DOES clamp
        let (x, y) = normalized_to_screen(1.5, -0.5, 0.0, 0.0, 1000.0, 1000.0);
        assert_eq!(x, 1000.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_relative_delta_with_rotation() {
        // Move 10mm right with 90 degree rotation should result in moving "down" in screen space
        let (dx, dy) = apply_relative_delta(10.0, 0.0, 0.0, 0.0, 90.0, 1.0, 1.0);
        // (10, 0) rotated 90 deg -> (0, 10)
        assert!(dx.abs() < 1e-6);
        assert!((dy - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_zero_area_normalization() {
        // Should not panic, even if results are Infinity or NaN
        let (u, v) = physical_to_normalized(50.0, 50.0, 50.0, 50.0, 0.0, 0.0, 0.0);
        assert!(u.is_nan() || u.is_infinite());
        assert!(v.is_nan() || v.is_infinite());
    }
}
