//! # Devocub Antichatter Filter
//!
//! An implementation of the popular "Devocub" smoothing algorithm. It uses a moving
//! average window (latency buffer) to eliminate high-frequency noise (chatter)
//! from hardware sensors, coupled with a linear prediction curve to compensate
//! for the latency introduced by the averaging.

use crate::core::config::models::MappingConfig;
use crate::filters::Filter;
use std::collections::VecDeque;

/// The Devocub hardware chatter reduction filter.
pub struct DevocubAntichatter {
    /// A limited-length ring buffer of past coordinates.
    history: VecDeque<(f32, f32)>,
    /// Running sum of `history`, kept in sync incrementally on push/pop so the
    /// average can be computed in O(1) instead of resumming the whole window
    /// on every packet.
    sum_x: f32,
    sum_y: f32,
    last_x: f32,
    last_y: f32,
}

impl DevocubAntichatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            history: VecDeque::new(),
            sum_x: 0.0,
            sum_y: 0.0,
            last_x: 0.0,
            last_y: 0.0,
        }
    }
}

impl Default for DevocubAntichatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for DevocubAntichatter {
    fn name(&self) -> &'static str {
        "Devocub Antichatter"
    }

    fn process(&mut self, x: f32, y: f32, config: &MappingConfig) -> (f32, f32) {
        let conf = &config.antichatter;
        if !conf.enabled {
            return (x, y);
        }

        // Latency buffering
        // We assume 1000Hz (1ms per sample) as per frequency setting
        // Window size = latency (ms) / (1000 / frequency)
        let window_size = (conf.latency * (conf.frequency / 1000.0)) as usize;
        let window_size = window_size.max(1);

        self.history.push_back((x, y));
        self.sum_x += x;
        self.sum_y += y;
        while self.history.len() > window_size {
            if let Some((ox, oy)) = self.history.pop_front() {
                self.sum_x -= ox;
                self.sum_y -= oy;
            }
        }

        // Simple averaging (basic antichatter)
        let avg_x = self.sum_x / self.history.len() as f32;
        let avg_y = self.sum_y / self.history.len() as f32;

        // Apply multiplier and offsets
        let mut out_x = avg_x.mul_add(
            conf.antichatter_multiplier,
            conf.antichatter_offset_x / 100.0,
        );
        let mut out_y = avg_y.mul_add(
            conf.antichatter_multiplier,
            conf.antichatter_offset_y / 100.0,
        );

        // Prediction (simplified)
        if conf.prediction_enabled
            && self.history.len() >= 2
            && let Some(&(px, py)) = self.history.iter().rev().nth(1)
        {
            let vx = x - px;
            let vy = y - py;

            out_x = (vx * conf.prediction_strength).mul_add(conf.prediction_sharpness, out_x);
            out_y = (vy * conf.prediction_strength).mul_add(conf.prediction_sharpness, out_y);
        }

        self.last_x = out_x;
        self.last_y = out_y;

        (out_x, out_y)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.sum_x = 0.0;
        self.sum_y = 0.0;
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
    use crate::core::config::models::MappingConfig;

    fn create_test_config(enabled: bool, latency: f32) -> MappingConfig {
        let mut config = MappingConfig::default();
        config.antichatter.enabled = enabled;
        config.antichatter.latency = latency;
        config.antichatter.frequency = 1000.0;
        config.antichatter.antichatter_multiplier = 1.0;
        config.antichatter.antichatter_offset_x = 0.0;
        config.antichatter.antichatter_offset_y = 0.0;
        config.antichatter.prediction_enabled = false;
        config
    }

    #[test]
    fn test_antichatter_disabled_passthrough() {
        let mut filter = DevocubAntichatter::new();
        let config = create_test_config(false, 10.0);

        let (x, y) = filter.process(0.5, 0.5, &config);
        assert!((x - 0.5).abs() < f32::EPSILON);
        assert!((y - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_antichatter_averaging() {
        let mut filter = DevocubAntichatter::new();
        // 2ms latency at 1000Hz = window of 2
        let config = create_test_config(true, 2.0);

        filter.process(0.0, 0.0, &config);
        let (x, y) = filter.process(1.0, 1.0, &config);

        // Average of (0,0) and (1,1) should be (0.5, 0.5)
        assert!((x - 0.5).abs() < f32::EPSILON);
        assert!((y - 0.5).abs() < f32::EPSILON);
    }
}
