//! Property-based tests for the coordinate pipeline.
//!
//! The unit tests next to the code check hand-picked points; these check the invariants that
//! must hold for any input: rotations preserve distances and compose, a point at the centre of
//! the active area always maps to the centre of the screen area, the screen projection never
//! leaves its target, and relative movement is linear and antisymmetric.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use next_tablet_driver::core::math::transform::{
    apply_relative_delta, normalized_to_screen, physical_to_normalized, rotate_point,
};
use proptest::prelude::*;

/// Absolute tolerance for `f32` results of a handful of trigonometric operations.
const EPS: f32 = 1e-3;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= EPS * (1.0 + a.abs().max(b.abs()))
}

fn coord() -> impl Strategy<Value = f32> {
    -10.0f32..10.0
}

fn angle() -> impl Strategy<Value = f32> {
    -720.0f32..720.0
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn rotation_by_zero_is_the_identity(x in coord(), y in coord(), cx in coord(), cy in coord()) {
        prop_assert_eq!(rotate_point(x, y, cx, cy, 0.0), (x, y));
    }

    #[test]
    fn rotation_keeps_the_distance_to_the_centre(
        x in coord(), y in coord(), cx in coord(), cy in coord(), degrees in angle(),
    ) {
        let (rx, ry) = rotate_point(x, y, cx, cy, degrees);
        let before = (x - cx).hypot(y - cy);
        let after = (rx - cx).hypot(ry - cy);
        prop_assert!(close(before, after), "{before} != {after}");
    }

    #[test]
    fn rotating_back_restores_the_point(
        x in coord(), y in coord(), cx in coord(), cy in coord(), degrees in angle(),
    ) {
        let (rx, ry) = rotate_point(x, y, cx, cy, degrees);
        let (bx, by) = rotate_point(rx, ry, cx, cy, -degrees);
        prop_assert!(close(bx, x) && close(by, y), "({bx}, {by}) != ({x}, {y})");
    }

    #[test]
    fn a_full_turn_restores_the_point(x in coord(), y in coord(), cx in coord(), cy in coord()) {
        let (rx, ry) = rotate_point(x, y, cx, cy, 360.0);
        prop_assert!(close(rx, x) && close(ry, y));
    }

    #[test]
    fn rotations_add_up(
        x in coord(), y in coord(), cx in coord(), cy in coord(), a in -360.0f32..360.0, b in -360.0f32..360.0,
    ) {
        let (ax, ay) = rotate_point(x, y, cx, cy, a);
        let (twice_x, twice_y) = rotate_point(ax, ay, cx, cy, b);
        let (once_x, once_y) = rotate_point(x, y, cx, cy, a + b);
        prop_assert!(close(twice_x, once_x) && close(twice_y, once_y));
    }

    #[test]
    fn the_centre_of_the_active_area_maps_to_the_centre_of_uv_space(
        area_x in 0.0f32..500.0, area_y in 0.0f32..500.0,
        width in 1.0f32..500.0, height in 1.0f32..500.0, rotation in angle(),
    ) {
        let (u, v) = physical_to_normalized(area_x, area_y, area_x, area_y, width, height, rotation);
        prop_assert!(close(u, 0.5) && close(v, 0.5), "({u}, {v})");
    }

    #[test]
    fn points_inside_the_active_area_land_inside_uv_space(
        area_x in 0.0f32..500.0, area_y in 0.0f32..500.0,
        width in 1.0f32..500.0, height in 1.0f32..500.0,
        fx in 0.0f32..=1.0, fy in 0.0f32..=1.0,
    ) {
        let x = (fx - 0.5).mul_add(width, area_x);
        let y = (fy - 0.5).mul_add(height, area_y);
        let (u, v) = physical_to_normalized(x, y, area_x, area_y, width, height, 0.0);
        prop_assert!((-EPS..=1.0 + EPS).contains(&u) && (-EPS..=1.0 + EPS).contains(&v), "({u}, {v})");
        prop_assert!(close(u, fx) && close(v, fy));
    }

    #[test]
    fn moving_right_increases_u_without_rotation(
        area_x in 0.0f32..500.0, width in 1.0f32..500.0, x in 0.0f32..500.0, step in 0.01f32..50.0,
    ) {
        let (u1, _) = physical_to_normalized(x, 0.0, area_x, 0.0, width, 100.0, 0.0);
        let (u2, _) = physical_to_normalized(x + step, 0.0, area_x, 0.0, width, 100.0, 0.0);
        prop_assert!(u2 > u1, "{u2} <= {u1}");
    }

    #[test]
    fn the_screen_projection_never_leaves_its_target(
        u in -5.0f32..5.0, v in -5.0f32..5.0,
        target_x in -3000.0f32..3000.0, target_y in -3000.0f32..3000.0,
        width in 1.0f32..8000.0, height in 1.0f32..8000.0,
    ) {
        let (x, y) = normalized_to_screen(u, v, target_x, target_y, width, height);
        prop_assert!(x >= target_x && x <= target_x + width, "x = {x}");
        prop_assert!(y >= target_y && y <= target_y + height, "y = {y}");
    }

    #[test]
    fn the_screen_projection_is_monotonic(
        a in -2.0f32..2.0, b in -2.0f32..2.0, width in 1.0f32..8000.0,
    ) {
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        let (x_low, _) = normalized_to_screen(low, 0.5, 0.0, 0.0, width, 100.0);
        let (x_high, _) = normalized_to_screen(high, 0.5, 0.0, 0.0, width, 100.0);
        prop_assert!(x_low <= x_high);
    }

    #[test]
    fn the_uv_corners_map_to_the_target_corners(
        target_x in -3000.0f32..3000.0, target_y in -3000.0f32..3000.0,
        width in 1.0f32..8000.0, height in 1.0f32..8000.0,
    ) {
        prop_assert_eq!(
            normalized_to_screen(0.0, 0.0, target_x, target_y, width, height),
            (target_x, target_y)
        );
        let (x, y) = normalized_to_screen(1.0, 1.0, target_x, target_y, width, height);
        prop_assert!(close(x, target_x + width) && close(y, target_y + height));
    }

    #[test]
    fn no_movement_means_no_delta(
        x in -200.0f32..200.0, y in -200.0f32..200.0, rotation in angle(),
        sens_x in 0.1f32..50.0, sens_y in 0.1f32..50.0,
    ) {
        let (dx, dy) = apply_relative_delta(x, y, x, y, rotation, sens_x, sens_y);
        prop_assert!(dx == 0.0 && dy == 0.0, "({dx}, {dy})");
    }

    #[test]
    fn unrotated_deltas_scale_with_the_sensitivity(
        x in -200.0f32..200.0, y in -200.0f32..200.0,
        last_x in -200.0f32..200.0, last_y in -200.0f32..200.0,
        sens_x in 0.1f32..50.0, sens_y in 0.1f32..50.0,
    ) {
        let (dx, dy) = apply_relative_delta(x, y, last_x, last_y, 0.0, sens_x, sens_y);
        prop_assert!(close(dx, (x - last_x) * sens_x) && close(dy, (y - last_y) * sens_y));
    }

    #[test]
    fn rotated_deltas_keep_their_length_with_a_uniform_sensitivity(
        x in -200.0f32..200.0, y in -200.0f32..200.0,
        last_x in -200.0f32..200.0, last_y in -200.0f32..200.0,
        rotation in angle(), sens in 0.1f32..50.0,
    ) {
        let (dx, dy) = apply_relative_delta(x, y, last_x, last_y, rotation, sens, sens);
        let expected = (x - last_x).hypot(y - last_y) * sens;
        prop_assert!(close(dx.hypot(dy), expected), "{} != {expected}", dx.hypot(dy));
    }

    #[test]
    fn moving_back_undoes_the_delta(
        x in -200.0f32..200.0, y in -200.0f32..200.0,
        last_x in -200.0f32..200.0, last_y in -200.0f32..200.0,
        rotation in angle(), sens_x in 0.1f32..50.0, sens_y in 0.1f32..50.0,
    ) {
        let (fx, fy) = apply_relative_delta(x, y, last_x, last_y, rotation, sens_x, sens_y);
        let (bx, by) = apply_relative_delta(last_x, last_y, x, y, rotation, sens_x, sens_y);
        prop_assert!(close(fx, -bx) && close(fy, -by), "({fx}, {fy}) vs ({bx}, {by})");
    }
}
