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
    last_x: f32,
    last_y: f32,
}

impl DevocubAntichatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            history: VecDeque::new(),
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

        // 1. Latency buffering
        // We assume 1000Hz (1ms per sample) as per frequency setting
        // Window size = latency (ms) / (1000 / frequency)
        let window_size = (conf.latency * (conf.frequency / 1000.0)) as usize;
        let window_size = window_size.max(1);

        self.history.push_back((x, y));
        while self.history.len() > window_size {
            self.history.pop_front();
        }

        // 2. Simple averaging (Basic Antichatter)
        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        for (hx, hy) in &self.history {
            avg_x += hx;
            avg_y += hy;
        }
        avg_x /= self.history.len() as f32;
        avg_y /= self.history.len() as f32;

        // 3. Apply Multiplier and Offsets
        let mut out_x = avg_x * conf.antichatter_multiplier + conf.antichatter_offset_x / 100.0;
        let mut out_y = avg_y * conf.antichatter_multiplier + conf.antichatter_offset_y / 100.0;

        // 4. Prediction (Simplified)
        if conf.prediction_enabled
            && self.history.len() >= 2
            && let Some(&(px, py)) = self.history.iter().rev().nth(1)
        {
            let vx = x - px;
            let vy = y - py;

            out_x += vx * conf.prediction_strength * conf.prediction_sharpness;
            out_y += vy * conf.prediction_strength * conf.prediction_sharpness;
        }

        self.last_x = out_x;
        self.last_y = out_y;

        (out_x, out_y)
    }

    fn reset(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
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
        assert_eq!(x, 0.5);
        assert_eq!(y, 0.5);
    }

    #[test]
    fn test_antichatter_averaging() {
        let mut filter = DevocubAntichatter::new();
        // 2ms latency at 1000Hz = window of 2
        let config = create_test_config(true, 2.0);

        filter.process(0.0, 0.0, &config);
        let (x, y) = filter.process(1.0, 1.0, &config);

        // Average of (0,0) and (1,1) should be (0.5, 0.5)
        assert_eq!(x, 0.5);
        assert_eq!(y, 0.5);
    }
}
