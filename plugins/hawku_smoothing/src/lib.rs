//! # Hawku Smoothing Plugin
//!
//! **Creator**: X9VoiD
//!
//! An implementation of the classic Hawku low-pass exponential smoothing filter.
//! Uses a target threshold decay calculation to provide smooth, latency-adjustable tracking.

use ntd_plugin_api::{
    NextTabletPlugin, PluginContext, PluginManifest, PluginPacket, PropertyDescriptor,
    PropertyKind, export_ntd_plugin,
};

/// Threshold factor for Hawku decay calculation (63% response target).
const HAWKU_THRESHOLD: f32 = 0.63_f32;

/// Standalone plugin implementing the Hawku Smoothing filter.
pub struct HawkuSmoothingPlugin {
    pub latency: f32,
    pub frequency: f32,

    last_pos: (f32, f32),
    last_timestamp_ns: u64,
    initialized: bool,
}

impl Default for HawkuSmoothingPlugin {
    fn default() -> Self {
        Self {
            latency: 2.0,
            frequency: 1000.0,
            last_pos: (0.0, 0.0),
            last_timestamp_ns: 0,
            initialized: false,
        }
    }
}

impl HawkuSmoothingPlugin {
    #[must_use]
    pub fn compute_weight(&self) -> f32 {
        let timer_interval = 1000.0_f32 / self.frequency.max(1.0_f32);
        let step_count = (self.latency / timer_interval).max(0.001_f32);
        let target = 1.0_f32 - HAWKU_THRESHOLD;
        let base = 1.0_f32 / target;
        1.0_f32 - (1.0_f32 / base.powf(1.0_f32 / step_count))
    }
}

impl NextTabletPlugin for HawkuSmoothingPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "hawku_smoothing".to_string(),
            name: "Hawku Smoothing".to_string(),
            version: "1.0.0".to_string(),
            author: "Hawku".to_string(),
            description: "Classic low-pass exponential smoothing filter designed to reduce cursor tremor while minimizing latency.".to_string(),
            icon: Some("LINE_SEGMENTS".to_string()),
            properties: vec![
                PropertyDescriptor {
                    id: "latency".to_string(),
                    name: "Latency".to_string(),
                    tooltip: Some("Smoothing latency in milliseconds (e.g. 15-25ms for Wacom-like feel, 2-10ms for snappy response).".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.1,
                        max: 500.0,
                        step: 0.5,
                        unit: "ms".to_string(),
                        default: 2.0,
                    },
                },
                PropertyDescriptor {
                    id: "frequency".to_string(),
                    name: "Filter Rate".to_string(),
                    tooltip: Some("Expected update frequency of input reports (typically 1000Hz).".to_string()),
                    kind: PropertyKind::Float {
                        min: 60.0,
                        max: 8000.0,
                        step: 10.0,
                        unit: "Hz".to_string(),
                        default: 1000.0,
                    },
                },
            ],
        }
    }

    fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext) {
        let width_mm = context.active_area_width_mm.max(0.1_f32);
        let height_mm = context.active_area_height_mm.max(0.1_f32);

        let target_x = packet.u * width_mm;
        let target_y = packet.v * height_mm;

        // If elapsed time is greater than 100ms (packet.timestamp_ns or fallback) or not initialized, snap immediately
        let time_delta_ms =
            if self.last_timestamp_ns > 0 && packet.timestamp_ns >= self.last_timestamp_ns {
                (packet.timestamp_ns - self.last_timestamp_ns) as f32 / 1_000_000.0_f32
            } else {
                0.0_f32
            };

        if !self.initialized
            || time_delta_ms > 100.0_f32
            || !self.last_pos.0.is_finite()
            || !self.last_pos.1.is_finite()
        {
            self.last_pos = (target_x, target_y);
            self.last_timestamp_ns = packet.timestamp_ns;
            self.initialized = true;
            return;
        }

        let weight = self.compute_weight().clamp(0.0_f32, 1.0_f32);
        let dx = target_x - self.last_pos.0;
        let dy = target_y - self.last_pos.1;

        self.last_pos.0 += dx * weight;
        self.last_pos.1 += dy * weight;
        self.last_timestamp_ns = packet.timestamp_ns;

        if !self.last_pos.0.is_finite() || !self.last_pos.1.is_finite() {
            self.last_pos = (target_x, target_y);
        }

        packet.u = (self.last_pos.0 / width_mm).clamp(0.0_f32, 1.0_f32);
        packet.v = (self.last_pos.1 / height_mm).clamp(0.0_f32, 1.0_f32);
    }

    fn set_property(&mut self, key: &str, value_json: &str) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(value_json) {
            match key {
                "latency" => {
                    if let Some(v) = val.as_f64() {
                        self.latency = (v as f32).clamp(0.01_f32, 2000.0_f32);
                    }
                }
                "frequency" => {
                    if let Some(v) = val.as_f64() {
                        self.frequency = (v as f32).clamp(1.0_f32, 20000.0_f32);
                    }
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self) {
        self.initialized = false;
        self.last_timestamp_ns = 0;
    }
}

export_ntd_plugin!(HawkuSmoothingPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hawku_initialization_passthrough() {
        let mut plugin = HawkuSmoothingPlugin::default();
        let mut packet = PluginPacket {
            u: 0.5,
            v: 0.5,
            timestamp_ns: 1_000_000,
            ..Default::default()
        };
        let ctx = PluginContext::default();

        plugin.process(&mut packet, &ctx);
        assert!((packet.u - 0.5).abs() < f32::EPSILON);
        assert!((packet.v - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hawku_smoothing_effect() {
        let mut plugin = HawkuSmoothingPlugin {
            latency: 20.0,
            frequency: 1000.0,
            ..Default::default()
        };
        let ctx = PluginContext::default();

        let mut p1 = PluginPacket {
            u: 0.0,
            v: 0.0,
            timestamp_ns: 1_000_000,
            ..Default::default()
        };
        plugin.process(&mut p1, &ctx);

        let mut p2 = PluginPacket {
            u: 1.0,
            v: 1.0,
            timestamp_ns: 2_000_000,
            ..Default::default()
        };
        plugin.process(&mut p2, &ctx);

        assert!(p2.u < 1.0);
        assert!(p2.u > 0.0);
    }

    #[test]
    fn test_hawku_jump_reset_after_100ms() {
        let mut plugin = HawkuSmoothingPlugin {
            latency: 20.0,
            frequency: 1000.0,
            ..Default::default()
        };
        let ctx = PluginContext::default();

        let mut p1 = PluginPacket {
            u: 0.0,
            v: 0.0,
            timestamp_ns: 1_000_000,
            ..Default::default()
        };
        plugin.process(&mut p1, &ctx);

        // 200ms gap -> should immediately snap to new position without smoothing lag
        let mut p2 = PluginPacket {
            u: 0.9,
            v: 0.9,
            timestamp_ns: 201_000_000,
            ..Default::default()
        };
        plugin.process(&mut p2, &ctx);

        assert!((p2.u - 0.9).abs() < 0.001);
    }
}
