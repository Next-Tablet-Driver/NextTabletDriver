//! # Devocub Antichatter Plugin
//!
//! **Creator**: X9VoiD
//!
//! An implementation of the Devocub smoothing and prediction algorithm.
//! Uses a latency-based weight curve combined with an antichatter curve and
//! hyperbolic secant (`1 / cosh`) prediction.

use ntd_plugin_api::{
    NextTabletPlugin, PluginContext, PluginManifest, PluginPacket, PropertyDescriptor,
    PropertyKind, export_ntd_plugin,
};

/// Standalone plugin implementing the Devocub Antichatter algorithm.
pub struct DevocubAntichatterPlugin {
    pub latency: f32,
    pub antichatter_strength: f32,
    pub antichatter_multiplier: f32,
    pub antichatter_offset_x: f32,
    pub antichatter_offset_y: f32,
    pub prediction_enabled: bool,
    pub prediction_strength: f32,
    pub prediction_sharpness: f32,
    pub prediction_offset_x: f32,
    pub prediction_offset_y: f32,

    current_pos: (f32, f32),
    prev_target_pos: (f32, f32),
    has_prev_target: bool,
    initialized: bool,
}

impl Default for DevocubAntichatterPlugin {
    fn default() -> Self {
        Self {
            latency: 2.0,
            antichatter_strength: 3.0,
            antichatter_multiplier: 1.0,
            antichatter_offset_x: 0.0,
            antichatter_offset_y: 1.0,
            prediction_enabled: false,
            prediction_strength: 1.1,
            prediction_sharpness: 1.0,
            prediction_offset_x: 3.0,
            prediction_offset_y: 0.3,

            current_pos: (0.0, 0.0),
            prev_target_pos: (0.0, 0.0),
            has_prev_target: false,
            initialized: false,
        }
    }
}

impl DevocubAntichatterPlugin {
    fn compute_base_weight(latency: f32) -> f32 {
        // Frequency assumed at 1000.0Hz -> timer_interval = 1.0ms
        let timer_interval = 1.0_f32;
        let step_count = (latency / timer_interval).max(0.001_f32);
        let target = 0.1_f32; // 1.0 - 0.9 (THRESHOLD in Devocub is 0.9)
        1.0_f32 - (1.0_f32 / (1.0_f32 / target).powf(1.0_f32 / step_count))
    }
}

impl NextTabletPlugin for DevocubAntichatterPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "devocub_antichatter".to_string(),
            name: "Devocub Antichatter".to_string(),
            version: "1.0.0".to_string(),
            author: "Devocub".to_string(),
            description: "Antichatter and prediction filter to eliminate sensor jitter while preserving cursor responsiveness.".to_string(),
            icon: Some("WAVEFORM".to_string()),
            properties: vec![
                PropertyDescriptor {
                    id: "latency".to_string(),
                    name: "Latency".to_string(),
                    tooltip: Some("Smoothing latency in milliseconds (higher = smoother, lower = faster).".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.5,
                        max: 200.0,
                        step: 0.5,
                        unit: "ms".to_string(),
                        default: 2.0,
                    },
                },
                PropertyDescriptor {
                    id: "antichatter_strength".to_string(),
                    name: "Antichatter Strength".to_string(),
                    tooltip: Some("Controls curve steepness. Higher values produce sharper response.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.5,
                        max: 20.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 3.0,
                    },
                },
                PropertyDescriptor {
                    id: "antichatter_multiplier".to_string(),
                    name: "Antichatter Multiplier".to_string(),
                    tooltip: Some("Scales smoothing sensitivity. Higher values make smoothing softer.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.1,
                        max: 1000.0,
                        step: 1.0,
                        unit: "".to_string(),
                        default: 1.0,
                    },
                },
                PropertyDescriptor {
                    id: "antichatter_offset_x".to_string(),
                    name: "Antichatter Offset X".to_string(),
                    tooltip: Some("Shifts the curve horizontally. Higher values activate smoothing earlier.".to_string()),
                    kind: PropertyKind::Float {
                        min: -5.0,
                        max: 5.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 0.0,
                    },
                },
                PropertyDescriptor {
                    id: "antichatter_offset_y".to_string(),
                    name: "Antichatter Offset Y".to_string(),
                    tooltip: Some("Minimum smoothing base level. 0 provides raw data, 1 is standard.".to_string()),
                    kind: PropertyKind::Float {
                        min: -5.0,
                        max: 10.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 1.0,
                    },
                },
                PropertyDescriptor {
                    id: "prediction".to_string(),
                    name: "Enable Prediction".to_string(),
                    tooltip: Some("Enables forward trajectory prediction to counteract smoothing latency.".to_string()),
                    kind: PropertyKind::Bool {
                        default: false,
                    },
                },
                PropertyDescriptor {
                    id: "prediction_strength".to_string(),
                    name: "Prediction Strength".to_string(),
                    tooltip: Some("Peak amplitude of the forward prediction compensation.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 5.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 1.1,
                    },
                },
                PropertyDescriptor {
                    id: "prediction_sharpness".to_string(),
                    name: "Prediction Sharpness".to_string(),
                    tooltip: Some("Width of the prediction peak along the velocity curve.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.1,
                        max: 5.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 1.0,
                    },
                },
                PropertyDescriptor {
                    id: "prediction_offset_x".to_string(),
                    name: "Prediction Offset X".to_string(),
                    tooltip: Some("Center of the prediction peak relative to cursor speed.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 10.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 3.0,
                    },
                },
                PropertyDescriptor {
                    id: "prediction_offset_y".to_string(),
                    name: "Prediction Offset Y".to_string(),
                    tooltip: Some("Minimum continuous prediction offset applied regardless of speed.".to_string()),
                    kind: PropertyKind::Float {
                        min: -5.0,
                        max: 5.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 0.3,
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

        let calc_target = if self.prediction_enabled {
            if !self.has_prev_target {
                self.prev_target_pos = (target_x, target_y);
                self.has_prev_target = true;
            }

            let delta_x = target_x - self.prev_target_pos.0;
            let delta_y = target_y - self.prev_target_pos.1;
            let distance = delta_x.hypot(delta_y);

            let cosh_val =
                ((distance - self.prediction_offset_x) * self.prediction_sharpness).cosh();
            let prediction_mod = if cosh_val.abs() > 0.0001_f32 {
                (1.0_f32 / cosh_val) * self.prediction_strength + self.prediction_offset_y
            } else {
                self.prediction_offset_y
            };

            self.prev_target_pos = (target_x, target_y);
            (
                target_x + delta_x * prediction_mod,
                target_y + delta_y * prediction_mod,
            )
        } else {
            (target_x, target_y)
        };

        if !self.initialized || !self.current_pos.0.is_finite() || !self.current_pos.1.is_finite() {
            self.current_pos = calc_target;
            self.initialized = true;
            return;
        }

        let delta_x = calc_target.0 - self.current_pos.0;
        let delta_y = calc_target.1 - self.current_pos.1;
        let distance = delta_x.hypot(delta_y);

        let base_weight = Self::compute_base_weight(self.latency);
        let dist_offset = (distance + self.antichatter_offset_x).max(0.0001_f32);
        let mut weight_modifier =
            dist_offset.powf(-self.antichatter_strength) * self.antichatter_multiplier;

        if weight_modifier + self.antichatter_offset_y < 0.0_f32 {
            weight_modifier = 0.0_f32;
        } else {
            weight_modifier += self.antichatter_offset_y;
        }

        let final_weight = if weight_modifier > 0.00001_f32 {
            (base_weight / weight_modifier).clamp(0.0_f32, 1.0_f32)
        } else {
            0.0_f32
        };

        self.current_pos.0 += delta_x * final_weight;
        self.current_pos.1 += delta_y * final_weight;

        if !self.current_pos.0.is_finite() || !self.current_pos.1.is_finite() {
            self.current_pos = calc_target;
        }

        packet.u = (self.current_pos.0 / width_mm).clamp(0.0_f32, 1.0_f32);
        packet.v = (self.current_pos.1 / height_mm).clamp(0.0_f32, 1.0_f32);
    }

    fn set_property(&mut self, key: &str, value_json: &str) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(value_json) {
            match key {
                "latency" => {
                    if let Some(v) = val.as_f64() {
                        self.latency = (v as f32).clamp(0.1_f32, 1000.0_f32);
                    }
                }
                "antichatter_strength" => {
                    if let Some(v) = val.as_f64() {
                        self.antichatter_strength = (v as f32).clamp(0.1_f32, 50.0_f32);
                    }
                }
                "antichatter_multiplier" => {
                    if let Some(v) = val.as_f64() {
                        self.antichatter_multiplier = (v as f32).clamp(0.01_f32, 10000.0_f32);
                    }
                }
                "antichatter_offset_x" => {
                    if let Some(v) = val.as_f64() {
                        self.antichatter_offset_x = (v as f32).clamp(-50.0_f32, 50.0_f32);
                    }
                }
                "antichatter_offset_y" => {
                    if let Some(v) = val.as_f64() {
                        self.antichatter_offset_y = (v as f32).clamp(-50.0_f32, 50.0_f32);
                    }
                }
                "prediction" => {
                    if let Some(v) = val.as_bool() {
                        self.prediction_enabled = v;
                    }
                }
                "prediction_strength" => {
                    if let Some(v) = val.as_f64() {
                        self.prediction_strength = (v as f32).clamp(0.0_f32, 20.0_f32);
                    }
                }
                "prediction_sharpness" => {
                    if let Some(v) = val.as_f64() {
                        self.prediction_sharpness = (v as f32).clamp(0.01_f32, 20.0_f32);
                    }
                }
                "prediction_offset_x" => {
                    if let Some(v) = val.as_f64() {
                        self.prediction_offset_x = (v as f32).clamp(0.0_f32, 50.0_f32);
                    }
                }
                "prediction_offset_y" => {
                    if let Some(v) = val.as_f64() {
                        self.prediction_offset_y = (v as f32).clamp(-50.0_f32, 50.0_f32);
                    }
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self) {
        self.initialized = false;
        self.has_prev_target = false;
    }
}

export_ntd_plugin!(DevocubAntichatterPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devocub_initialization_passthrough() {
        let mut plugin = DevocubAntichatterPlugin::default();
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
    fn test_devocub_smoothing_pull() {
        let mut plugin = DevocubAntichatterPlugin {
            latency: 20.0,
            ..Default::default()
        };
        let ctx = PluginContext::default();

        let mut packet1 = PluginPacket {
            u: 0.0,
            v: 0.0,
            ..Default::default()
        };
        plugin.process(&mut packet1, &ctx);

        let mut packet2 = PluginPacket {
            u: 1.0,
            v: 1.0,
            ..Default::default()
        };
        plugin.process(&mut packet2, &ctx);

        // Due to latency smoothing, cursor shouldn't immediately reach 1.0
        assert!(packet2.u < 1.0);
        assert!(packet2.u > 0.0);
    }

    #[test]
    fn test_devocub_property_update() {
        let mut plugin = DevocubAntichatterPlugin::default();
        plugin.set_property("latency", "15.5");
        assert!((plugin.latency - 15.5).abs() < 0.001);

        plugin.set_property("prediction", "true");
        assert!(plugin.prediction_enabled);
    }
}
