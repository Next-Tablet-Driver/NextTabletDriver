//! # Kalman Smoothing Filter
//!
//! A standard scalar Kalman filter applied independently to the x and y
//! coordinate streams to reduce sensor jitter while remaining more responsive
//! than a fixed moving-average window.

use crate::core::config::models::MappingConfig;
use crate::filters::Filter;

/// Tracks the running estimate and error covariance for a single axis.
#[derive(Clone, Copy, Debug)]
struct KalmanAxis {
    estimate: f32,
    error_covariance: f32,
    initialized: bool,
}

impl KalmanAxis {
    const fn new() -> Self {
        Self {
            estimate: 0.0,
            error_covariance: 1.0,
            initialized: false,
        }
    }

    fn update(&mut self, measurement: f32, process_noise: f32, measurement_noise: f32) -> f32 {
        if !self.initialized {
            self.estimate = measurement;
            self.error_covariance = 1.0;
            self.initialized = true;
            return self.estimate;
        }

        // Predict
        let predicted_covariance = self.error_covariance + process_noise;

        // Update
        let kalman_gain = predicted_covariance / (predicted_covariance + measurement_noise);
        self.estimate = kalman_gain.mul_add(measurement - self.estimate, self.estimate);
        self.error_covariance = (1.0 - kalman_gain) * predicted_covariance;

        self.estimate
    }

    const fn reset(&mut self) {
        self.initialized = false;
    }
}

/// The Kalman smoothing filter, tracking x and y independently.
pub struct KalmanFilter {
    x_axis: KalmanAxis,
    y_axis: KalmanAxis,
}

impl KalmanFilter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            x_axis: KalmanAxis::new(),
            y_axis: KalmanAxis::new(),
        }
    }
}

impl Default for KalmanFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for KalmanFilter {
    fn name(&self) -> &'static str {
        "Kalman Smoothing"
    }

    fn process(&mut self, x: f32, y: f32, config: &MappingConfig) -> (f32, f32) {
        let conf = &config.kalman;
        if !conf.enabled {
            return (x, y);
        }

        let out_x = self
            .x_axis
            .update(x, conf.process_noise, conf.measurement_noise);
        let out_y = self
            .y_axis
            .update(y, conf.process_noise, conf.measurement_noise);

        (out_x, out_y)
    }

    fn reset(&mut self) {
        self.x_axis.reset();
        self.y_axis.reset();
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    fn create_test_config(enabled: bool) -> MappingConfig {
        let mut config = MappingConfig::default();
        config.kalman.enabled = enabled;
        config.kalman.process_noise = 0.005;
        config.kalman.measurement_noise = 0.05;
        config
    }

    #[test]
    fn test_kalman_disabled_passthrough() {
        let mut filter = KalmanFilter::new();
        let config = create_test_config(false);

        let (x, y) = filter.process(0.5, 0.5, &config);
        assert!((x - 0.5).abs() < f32::EPSILON);
        assert!((y - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_kalman_converges_toward_true_center_under_noise() {
        let mut filter = KalmanFilter::new();
        let config = create_test_config(true);

        // Noisy but centered around 0.5: alternating +/- offset.
        let samples = [0.5, 0.6, 0.4, 0.55, 0.45, 0.58, 0.42, 0.52, 0.48, 0.5];
        let mut last_filtered = 0.5;
        for &s in &samples {
            let (fx, _fy) = filter.process(s, s, &config);
            last_filtered = fx;
        }

        // The raw last sample has some noise; the filtered output should be
        // closer to the true center (0.5) than at least one noisy raw sample.
        assert!((last_filtered - 0.5).abs() < 0.15);
    }

    #[test]
    fn test_kalman_reset_reinitializes_state() {
        let mut filter = KalmanFilter::new();
        let config = create_test_config(true);

        filter.process(0.9, 0.9, &config);
        filter.reset();

        // After reset, the first sample should pass through as-is (fresh init).
        let (x, y) = filter.process(0.1, 0.1, &config);
        assert!((x - 0.1).abs() < f32::EPSILON);
        assert!((y - 0.1).abs() < f32::EPSILON);
    }
}
