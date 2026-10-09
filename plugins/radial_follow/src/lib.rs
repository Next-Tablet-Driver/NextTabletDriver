//! # Radial Follow Smoothing Plugin
//!
//! **Creator**: AbstractQbit
//!
//! Smooths pen cursor movement within an inner and outer radial boundary using a soft-knee transition.

use ntd_plugin_api::{
    NextTabletPlugin, PluginContext, PluginManifest, PluginPacket, PropertyDescriptor,
    PropertyKind, export_ntd_plugin,
};

/// The coordinate space in which radial follow smoothing calculations occur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RadialSpace {
    /// Calculations operate in screen pixel coordinates.
    #[default]
    Screen = 0,
    /// Calculations operate in physical tablet millimeter coordinates.
    Tablet = 1,
}

/// Standalone plugin implementing the Radial Follow algorithm.
pub struct RadialFollowPlugin {
    pub outer_radius: f64,
    pub inner_radius: f64,
    pub smoothing_coefficient: f64,
    pub soft_knee_scale: f64,
    pub smoothing_leak_coefficient: f64,
    pub space: RadialSpace,

    cursor_x: f64,
    cursor_y: f64,
    initialized: bool,
}

impl Default for RadialFollowPlugin {
    fn default() -> Self {
        Self {
            outer_radius: 5.0,
            inner_radius: 0.0,
            smoothing_coefficient: 0.95,
            soft_knee_scale: 1.0,
            smoothing_leak_coefficient: 0.0,
            space: RadialSpace::Screen,
            cursor_x: 0.0,
            cursor_y: 0.0,
            initialized: false,
        }
    }
}

impl RadialFollowPlugin {
    fn knee_func(x: f64) -> f64 {
        if x < -3.0 {
            x
        } else if x < 3.0 {
            (x.exp().tanh()).ln()
        } else {
            0.0
        }
    }

    fn knee_scaled(x: f64, kn_scale: f64) -> f64 {
        if kn_scale > 0.0001 {
            kn_scale * Self::knee_func(x / kn_scale) + 1.0
        } else if x > 0.0 {
            1.0
        } else {
            1.0 + x
        }
    }

    fn inverse_tanh(x: f64) -> f64 {
        ((1.0 + x) / (1.0 - x)).ln() / 2.0
    }

    fn inverse_knee_scaled(x: f64, kn_scale: f64) -> f64 {
        kn_scale * (Self::inverse_tanh(((x - 1.0) / kn_scale).exp())).ln()
    }

    fn deriv_knee_scaled(x: f64, kn_scale: f64) -> f64 {
        let e = (x / kn_scale).exp();
        let tanh = e.tanh();
        (e - e * (tanh * tanh)) / tanh
    }

    fn get_x_offset(kn_scale: f64) -> f64 {
        if kn_scale > 0.0001 {
            Self::inverse_knee_scaled(0.0, kn_scale)
        } else {
            -1.0
        }
    }

    fn get_scale_comp(kn_scale: f64, x_offset: f64) -> f64 {
        if kn_scale > 0.0001 {
            Self::deriv_knee_scaled(x_offset, kn_scale)
        } else {
            1.0
        }
    }

    fn sample_radial_curve(&self, dist: f64) -> f64 {
        let r_outer_adj = self.outer_radius.max(self.inner_radius + 0.0001);
        let r_inner_adj = self.inner_radius;
        let delta_r = r_outer_adj - r_inner_adj;

        if dist <= r_inner_adj {
            return 0.0;
        }

        let x_offset = Self::get_x_offset(self.soft_knee_scale);
        let scale_comp = Self::get_scale_comp(self.soft_knee_scale, x_offset);

        let leaked_fn = |x: f64| -> f64 {
            Self::knee_scaled(x + x_offset, self.soft_knee_scale)
                * (1.0 - self.smoothing_leak_coefficient)
                + x * self.smoothing_leak_coefficient * scale_comp
        };

        let smoothed_fn =
            |x: f64| -> f64 { leaked_fn(x * self.smoothing_coefficient / scale_comp) };

        let scale_to_outer = |x: f64| -> f64 { delta_r * smoothed_fn(x / delta_r) };

        dist - scale_to_outer(dist - r_inner_adj) - r_inner_adj
    }

    fn filter_position(&mut self, target_x: f64, target_y: f64) -> (f64, f64) {
        if !self.initialized || !self.cursor_x.is_finite() || !self.cursor_y.is_finite() {
            self.cursor_x = target_x;
            self.cursor_y = target_y;
            self.initialized = true;
            return (target_x, target_y);
        }

        let dx = target_x - self.cursor_x;
        let dy = target_y - self.cursor_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > 0.000001 {
            let dist_to_move = self.sample_radial_curve(dist);
            let dir_x = dx / dist;
            let dir_y = dy / dist;
            self.cursor_x += dir_x * dist_to_move;
            self.cursor_y += dir_y * dist_to_move;
        }

        if !self.cursor_x.is_finite() || !self.cursor_y.is_finite() {
            self.cursor_x = target_x;
            self.cursor_y = target_y;
        }

        (self.cursor_x, self.cursor_y)
    }
}

impl NextTabletPlugin for RadialFollowPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "radial_follow".to_string(),
            name: "Radial Follow Smoothing".to_string(),
            version: "1.0.0".to_string(),
            author: "AbstractQbit".to_string(),
            description: "Smooths pen cursor movement within an inner and outer radial boundary using a soft-knee transition.".to_string(),
            icon: Some("🎯".to_string()),
            properties: vec![
                PropertyDescriptor {
                    id: "outer_radius".to_string(),
                    name: "Outer Radius".to_string(),
                    tooltip: Some("Maximum distance the cursor can lag behind reading before catching up rapidly.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 500.0,
                        step: 0.5,
                        unit: "px/mm".to_string(),
                        default: 5.0,
                    },
                },
                PropertyDescriptor {
                    id: "inner_radius".to_string(),
                    name: "Inner Radius".to_string(),
                    tooltip: Some("Deadzone distance within which input deviations produce no cursor movement.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 500.0,
                        step: 0.5,
                        unit: "px/mm".to_string(),
                        default: 0.0,
                    },
                },
                PropertyDescriptor {
                    id: "smoothing_coefficient".to_string(),
                    name: "Smoothing Coefficient".to_string(),
                    tooltip: Some("Determines how fast the cursor descends from outer radius to inner radius (higher = smoother).".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.01,
                        unit: "".to_string(),
                        default: 0.95,
                    },
                },
                PropertyDescriptor {
                    id: "soft_knee_scale".to_string(),
                    name: "Soft Knee Scale".to_string(),
                    tooltip: Some("Controls transition softness around the outer radius boundary (higher = softer).".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 100.0,
                        step: 0.1,
                        unit: "".to_string(),
                        default: 1.0,
                    },
                },
                PropertyDescriptor {
                    id: "smoothing_leak_coefficient".to_string(),
                    name: "Smoothing Leak Coefficient".to_string(),
                    tooltip: Some("Allows smoothing to persist past the outer radius at a reduced rate.".to_string()),
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.01,
                        unit: "".to_string(),
                        default: 0.0,
                    },
                },
                PropertyDescriptor {
                    id: "space".to_string(),
                    name: "Coordinate Space".to_string(),
                    tooltip: Some("Selects whether the radius boundaries are evaluated in Screen pixels or Tablet millimeters.".to_string()),
                    kind: PropertyKind::Choice {
                        options: vec!["Screen Space (px)".to_string(), "Tablet Space (mm)".to_string()],
                        default_index: 0,
                    },
                },
            ],
        }
    }

    fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext) {
        let (target_x, target_y, scale_x, scale_y) = match self.space {
            RadialSpace::Screen => {
                let w = f64::from(context.target_area_width_px.max(1.0));
                let h = f64::from(context.target_area_height_px.max(1.0));
                (f64::from(packet.u) * w, f64::from(packet.v) * h, w, h)
            }
            RadialSpace::Tablet => {
                let w = f64::from(context.active_area_width_mm.max(0.1));
                let h = f64::from(context.active_area_height_mm.max(0.1));
                (f64::from(packet.u) * w, f64::from(packet.v) * h, w, h)
            }
        };

        let (filtered_x, filtered_y) = self.filter_position(target_x, target_y);

        packet.u = (filtered_x / scale_x).clamp(0.0, 1.0) as f32;
        packet.v = (filtered_y / scale_y).clamp(0.0, 1.0) as f32;
    }

    fn set_property(&mut self, key: &str, value_json: &str) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(value_json) {
            match key {
                "outer_radius" => {
                    if let Some(v) = val.as_f64() {
                        self.outer_radius = v.clamp(0.0, 5000.0);
                    }
                }
                "inner_radius" => {
                    if let Some(v) = val.as_f64() {
                        self.inner_radius = v.clamp(0.0, 5000.0);
                    }
                }
                "smoothing_coefficient" => {
                    if let Some(v) = val.as_f64() {
                        self.smoothing_coefficient = v.clamp(0.0, 1.0);
                    }
                }
                "soft_knee_scale" => {
                    if let Some(v) = val.as_f64() {
                        self.soft_knee_scale = v.clamp(0.0, 100.0);
                    }
                }
                "smoothing_leak_coefficient" => {
                    if let Some(v) = val.as_f64() {
                        self.smoothing_leak_coefficient = v.clamp(0.0, 1.0);
                    }
                }
                "space" => {
                    if let Some(v) = val.as_u64() {
                        self.space = if v == 1 {
                            RadialSpace::Tablet
                        } else {
                            RadialSpace::Screen
                        };
                    }
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self) {
        self.initialized = false;
    }
}

export_ntd_plugin!(RadialFollowPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radial_follow_initialization_passthrough() {
        let mut plugin = RadialFollowPlugin::default();
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
    fn test_radial_follow_outer_radius_zero_property() {
        let mut plugin = RadialFollowPlugin::default();
        plugin.set_property("outer_radius", "0.0");
        assert_eq!(plugin.outer_radius, 0.0);

        plugin.set_property("outer_radius", "-10.0");
        assert_eq!(plugin.outer_radius, 0.0);
    }

    #[test]
    fn test_radial_follow_outer_radius_descriptor_min_is_zero() {
        let plugin = RadialFollowPlugin::default();
        let manifest = plugin.manifest();
        let outer_radius_prop = manifest
            .properties
            .iter()
            .find(|p| p.id == "outer_radius")
            .expect("outer_radius property should exist");

        match outer_radius_prop.kind {
            PropertyKind::Float { min, .. } => {
                assert_eq!(min, 0.0);
            }
            _ => panic!("Expected Float property kind for outer_radius"),
        }
    }

    #[test]
    fn test_radial_follow_process_with_outer_radius_zero() {
        let mut plugin = RadialFollowPlugin::default();
        plugin.set_property("outer_radius", "0.0");
        let ctx = PluginContext::default();

        let mut p1 = PluginPacket {
            u: 0.1,
            v: 0.1,
            ..Default::default()
        };
        plugin.process(&mut p1, &ctx);

        let mut p2 = PluginPacket {
            u: 0.9,
            v: 0.9,
            ..Default::default()
        };
        plugin.process(&mut p2, &ctx);

        assert!(p2.u.is_finite());
        assert!(p2.v.is_finite());
        assert!((p2.u - 0.9).abs() < 0.01);
        assert!((p2.v - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_radial_follow_smoothing_coefficient_zero_property() {
        let mut plugin = RadialFollowPlugin::default();
        plugin.set_property("smoothing_coefficient", "0.0");
        assert_eq!(plugin.smoothing_coefficient, 0.0);

        plugin.set_property("smoothing_coefficient", "-0.5");
        assert_eq!(plugin.smoothing_coefficient, 0.0);
    }

    #[test]
    fn test_radial_follow_smoothing_coefficient_descriptor_min_is_zero() {
        let plugin = RadialFollowPlugin::default();
        let manifest = plugin.manifest();
        let smoothing_prop = manifest
            .properties
            .iter()
            .find(|p| p.id == "smoothing_coefficient")
            .expect("smoothing_coefficient property should exist");

        match smoothing_prop.kind {
            PropertyKind::Float { min, .. } => {
                assert_eq!(min, 0.0);
            }
            _ => panic!("Expected Float property kind for smoothing_coefficient"),
        }
    }

    #[test]
    fn test_radial_follow_process_with_smoothing_coefficient_zero() {
        let mut plugin = RadialFollowPlugin::default();
        plugin.set_property("smoothing_coefficient", "0.0");
        let ctx = PluginContext::default();

        let mut p1 = PluginPacket {
            u: 0.1,
            v: 0.1,
            ..Default::default()
        };
        plugin.process(&mut p1, &ctx);

        let mut p2 = PluginPacket {
            u: 0.9,
            v: 0.9,
            ..Default::default()
        };
        plugin.process(&mut p2, &ctx);

        assert!(p2.u.is_finite());
        assert!(p2.v.is_finite());
        assert!((p2.u - 0.9).abs() < 0.01);
        assert!((p2.v - 0.9).abs() < 0.01);
    }
}
