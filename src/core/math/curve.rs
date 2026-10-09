//! Pressure response curve evaluation.
//!
//! Reshapes normalized pressure input `[0, 1]` before it's reported to the
//! OS/game. Purely a function of the already-normalized ratio; never touches
//! raw pressure used for tip-down/up threshold detection.

use crate::core::config::models::{PressureCurveConfig, PressureCurveType};

/// Identity curve: output equals input.
#[must_use]
pub const fn evaluate_linear(t: f32) -> f32 {
    t
}

/// Exponential curve: `input.powf(exponent)`.
#[must_use]
pub fn evaluate_exponential(t: f32, exponent: f32) -> f32 {
    t.powf(exponent)
}

/// Piecewise-linear interpolation through sorted control points.
///
/// Falls back to the identity curve if fewer than 2 points are provided.
#[must_use]
pub fn evaluate_custom(t: f32, points: &[(f32, f32)]) -> f32 {
    let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
        return t;
    };
    if points.len() < 2 {
        return t;
    }

    if t <= first.0 {
        return first.1;
    }
    if t >= last.0 {
        return last.1;
    }

    for pair in points.windows(2) {
        if let [(x0, y0), (x1, y1)] = *pair
            && t >= x0
            && t <= x1
        {
            let span = x1 - x0;
            if span <= f32::EPSILON {
                return y1;
            }
            let ratio = (t - x0) / span;
            return (y1 - y0).mul_add(ratio, y0);
        }
    }

    t
}

/// Evaluates the configured pressure curve at `t`, clamping both input and
/// output to `[0, 1]`.
#[must_use]
pub fn evaluate(t: f32, config: &PressureCurveConfig) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let out = match config.curve_type {
        PressureCurveType::Linear => evaluate_linear(t),
        PressureCurveType::Exponential => evaluate_exponential(t, config.exponent),
        PressureCurveType::Custom => evaluate_custom(t, &config.points),
    };
    out.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_is_identity() {
        for i in 0..=10u16 {
            let t = f32::from(i) / 10.0;
            assert!((evaluate_linear(t) - t).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_exponential_preserves_endpoints_and_is_monotonic() {
        let exponent = 2.5;
        assert!((evaluate_exponential(0.0, exponent) - 0.0).abs() < f32::EPSILON);
        assert!((evaluate_exponential(1.0, exponent) - 1.0).abs() < f32::EPSILON);

        let mut prev = evaluate_exponential(0.0, exponent);
        for i in 1..=20u16 {
            let t = f32::from(i) / 20.0;
            let cur = evaluate_exponential(t, exponent);
            assert!(cur >= prev);
            prev = cur;
        }
    }

    #[test]
    fn test_custom_exact_points() {
        let points = vec![(0.0, 0.0), (0.5, 0.8), (1.0, 1.0)];
        assert!((evaluate_custom(0.0, &points) - 0.0).abs() < f32::EPSILON);
        assert!((evaluate_custom(0.5, &points) - 0.8).abs() < f32::EPSILON);
        assert!((evaluate_custom(1.0, &points) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_custom_interpolates_between_points() {
        let points = vec![(0.0, 0.0), (1.0, 1.0)];
        assert!((evaluate_custom(0.25, &points) - 0.25).abs() < f32::EPSILON);
        assert!((evaluate_custom(0.75, &points) - 0.75).abs() < f32::EPSILON);

        let steep = vec![(0.0, 0.0), (0.5, 0.9), (1.0, 1.0)];
        let mid = evaluate_custom(0.25, &steep);
        assert!((mid - 0.45).abs() < 1e-5);
    }

    #[test]
    fn test_custom_degenerate_falls_back_to_linear() {
        let points = vec![(0.5, 0.9)];
        assert!((evaluate_custom(0.3, &points) - 0.3).abs() < f32::EPSILON);

        let empty: Vec<(f32, f32)> = vec![];
        assert!((evaluate_custom(0.3, &empty) - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_evaluate_dispatch_clamps_input_output() {
        let config = PressureCurveConfig {
            curve_type: PressureCurveType::Linear,
            ..Default::default()
        };
        assert!((evaluate(-0.5, &config) - 0.0).abs() < f32::EPSILON);
        assert!((evaluate(1.5, &config) - 1.0).abs() < f32::EPSILON);
    }
}
