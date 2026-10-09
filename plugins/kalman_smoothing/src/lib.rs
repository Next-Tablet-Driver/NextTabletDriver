//! # Kalman Smoothing Plugin
//!
//! **Creator**: iSweat
//!
//! A discrete Kalman filter applied independently to the horizontal and vertical
//! coordinate streams to reduce sensor noise while maximizing responsiveness.

use ntd_plugin_api::{
    NextTabletPlugin, PluginContext, PluginManifest, PluginPacket, PropertyDescriptor,
    PropertyKind, export_ntd_plugin,
};

/// Tracks the running estimate and error covariance for a single coordinate axis.
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
        if !self.initialized || !self.estimate.is_finite() {
            self.estimate = measurement;
            self.error_covariance = 1.0;
            self.initialized = true;
            return self.estimate;
        }

        let predicted_covariance = self.error_covariance + process_noise;

        let denominator = predicted_covariance + measurement_noise;
        let kalman_gain = if denominator.abs() > 0.00001_f32 {
            predicted_covariance / denominator
        } else {
            0.5_f32
        };

        self.estimate += kalman_gain * (measurement - self.estimate);
        self.error_covariance = (1.0_f32 - kalman_gain) * predicted_covariance;

        if !self.estimate.is_finite() {
            self.estimate = measurement;
        }

        self.estimate
    }

    const fn reset(&mut self) {
        self.initialized = false;
    }
}

/// Standalone plugin implementing the Kalman Smoothing filter.
pub struct KalmanSmoothingPlugin {
    pub process_noise: f32,
    pub measurement_noise: f32,
    x_axis: KalmanAxis,
    y_axis: KalmanAxis,
}

impl Default for KalmanSmoothingPlugin {
    fn default() -> Self {
        Self {
            process_noise: 0.005,
            measurement_noise: 0.05,
            x_axis: KalmanAxis::new(),
            y_axis: KalmanAxis::new(),
        }
    }
}

impl NextTabletPlugin for KalmanSmoothingPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "kalman_smoothing".to_string(),
            name: "Kalman Smoothing".to_string(),
            version: "1.0.0".to_string(),
            author: "OpenTabletDriver Community".to_string(),
            description: "Discrete dual-axis Kalman filter reducing hardware noise with minimal phase delay.".to_string(),
            icon: Some("CHART_LINE".to_string()),
            properties: vec![
                PropertyDescriptor {
                    id: "process_noise".to_string(),
                    name: "Process Noise (Q)".to_string(),
                    tooltip: Some("How rapidly the true pen trajectory is expected to change (higher = faster response).".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0001,
                        max: 1.0,
                        step: 0.001,
                        unit: "".to_string(),
                        default: 0.005,
                    },
                },
                PropertyDescriptor {
                    id: "measurement_noise".to_string(),
                    name: "Measurement Noise (R)".to_string(),
                    tooltip: Some("Estimated hardware jitter level (higher = stronger noise filtering).".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0001,
                        max: 1.0,
                        step: 0.005,
                        unit: "".to_string(),
                        default: 0.05,
                    },
                },
            ],
        }
    }

    fn process(&mut self, packet: &mut PluginPacket, _context: &PluginContext) {
        packet.u = self
            .x_axis
            .update(packet.u, self.process_noise, self.measurement_noise)
            .clamp(0.0_f32, 1.0_f32);
        packet.v = self
            .y_axis
            .update(packet.v, self.process_noise, self.measurement_noise)
            .clamp(0.0_f32, 1.0_f32);
    }

    fn set_property(&mut self, key: &str, value_json: &str) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(value_json) {
            match key {
                "process_noise" => {
                    if let Some(v) = val.as_f64() {
                        self.process_noise = (v as f32).clamp(0.00001_f32, 10.0_f32);
                    }
                }
                "measurement_noise" => {
                    if let Some(v) = val.as_f64() {
                        self.measurement_noise = (v as f32).clamp(0.00001_f32, 10.0_f32);
                    }
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self) {
        self.x_axis.reset();
        self.y_axis.reset();
    }
}

export_ntd_plugin!(KalmanSmoothingPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_initialization_passthrough() {
        let mut plugin = KalmanSmoothingPlugin::default();
        let mut packet = PluginPacket {
            u: 0.5,
            v: 0.5,
            ..Default::default()
        };
        let ctx = PluginContext::default();

        plugin.process(&mut packet, &ctx);
        assert!((packet.u - 0.5).abs() < f32::EPSILON);
        assert!((packet.v - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_kalman_smoothing_convergence() {
        let mut plugin = KalmanSmoothingPlugin::default();
        let ctx = PluginContext::default();

        let samples = [0.5, 0.6, 0.4, 0.55, 0.45, 0.58, 0.42, 0.52, 0.48, 0.5];
        let mut last_filtered = 0.5;
        for &s in &samples {
            let mut packet = PluginPacket {
                u: s,
                v: s,
                ..Default::default()
            };
            plugin.process(&mut packet, &ctx);
            last_filtered = packet.u;
        }

        assert!((last_filtered - 0.5).abs() < 0.15);
    }

    #[test]
    fn test_kalman_reset() {
        let mut plugin = KalmanSmoothingPlugin::default();
        let ctx = PluginContext::default();

        let mut p1 = PluginPacket {
            u: 0.9,
            v: 0.9,
            ..Default::default()
        };
        plugin.process(&mut p1, &ctx);

        plugin.reset();

        let mut p2 = PluginPacket {
            u: 0.1,
            v: 0.1,
            ..Default::default()
        };
        plugin.process(&mut p2, &ctx);
        assert!((p2.u - 0.1).abs() < f32::EPSILON);
    }
}
