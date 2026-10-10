//! Example NextTabletDriver plugin: a simple smoothing filter.
//!
//! Each packet moves the position only part of the way from the previous position to the new
//! one. A higher `strength` gives a smoother but slower cursor.
//!
//! Copy this folder to start your own plugin. The guide is `docs/plugin-development.md`.

use ntd_plugin_api::{
    NextTabletPlugin, PluginContext, PluginManifest, PluginPacket, PropertyDescriptor,
    PropertyKind, export_ntd_plugin,
};

/// The range of the `strength` setting. Used by the manifest and by `set_property`, so the two
/// can never disagree.
const STRENGTH_MIN: f32 = 0.0;
const STRENGTH_MAX: f32 = 0.95;
const STRENGTH_DEFAULT: f32 = 0.5;

/// The plugin state. The engine creates one instance with `Default::default()`.
pub struct SimpleSmoothing {
    strength: f32,
    /// The position of the previous packet, or `None` before the first packet and after a reset.
    last: Option<(f32, f32)>,
}

impl Default for SimpleSmoothing {
    fn default() -> Self {
        Self {
            strength: STRENGTH_DEFAULT,
            last: None,
        }
    }
}

impl NextTabletPlugin for SimpleSmoothing {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            // Keep the id stable: users' settings are saved under it.
            id: "simple_smoothing".to_string(),
            name: "Simple Smoothing".to_string(),
            version: "1.0.0".to_string(),
            author: "You".to_string(),
            description: "Blends each position with the previous one.".to_string(),
            icon: None,
            properties: vec![PropertyDescriptor {
                id: "strength".to_string(),
                name: "Strength".to_string(),
                tooltip: Some("0 = no smoothing, 0.95 = very smooth".to_string()),
                kind: PropertyKind::Float {
                    min: STRENGTH_MIN,
                    max: STRENGTH_MAX,
                    step: 0.01,
                    unit: String::new(),
                    default: STRENGTH_DEFAULT,
                },
            }],
        }
    }

    /// Runs for every packet. Keep it short: no allocation, no locks, no I/O.
    fn process(&mut self, packet: &mut PluginPacket, _context: &PluginContext) {
        if let Some((last_u, last_v)) = self.last {
            let follow = 1.0 - self.strength;
            packet.u = last_u + (packet.u - last_u) * follow;
            packet.v = last_v + (packet.v - last_v) * follow;
        }
        self.last = Some((packet.u, packet.v));
    }

    /// Can be called often, with the same value, and with values you did not expect.
    fn set_property(&mut self, key: &str, value_json: &str) {
        if key == "strength"
            && let Ok(value) = serde_json::from_str::<f32>(value_json)
            && value.is_finite()
        {
            self.strength = value.clamp(STRENGTH_MIN, STRENGTH_MAX);
        }
    }

    /// Forget the running state. Can be called many times in a row.
    fn reset(&mut self) {
        self.last = None;
    }
}

export_ntd_plugin!(SimpleSmoothing);

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(u: f32, v: f32) -> PluginPacket {
        PluginPacket {
            u,
            v,
            ..PluginPacket::default()
        }
    }

    #[test]
    fn the_first_packet_passes_unchanged() {
        let mut plugin = SimpleSmoothing::default();
        let mut p = packet(0.4, 0.6);
        plugin.process(&mut p, &PluginContext::default());
        assert_eq!((p.u, p.v), (0.4, 0.6));
    }

    #[test]
    fn a_jump_is_only_followed_part_of_the_way() {
        let mut plugin = SimpleSmoothing::default(); // strength 0.5
        let context = PluginContext::default();
        plugin.process(&mut packet(0.0, 0.0), &context);

        let mut p = packet(1.0, 1.0);
        plugin.process(&mut p, &context);
        assert!((p.u - 0.5).abs() < 1e-6);
        assert!((p.v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_reset_forgets_the_previous_position() {
        let mut plugin = SimpleSmoothing::default();
        let context = PluginContext::default();
        plugin.process(&mut packet(0.0, 0.0), &context);
        plugin.reset();
        plugin.reset(); // calling it again must be harmless

        let mut p = packet(1.0, 1.0);
        plugin.process(&mut p, &context);
        assert_eq!((p.u, p.v), (1.0, 1.0));
    }

    #[test]
    fn the_setting_is_clamped_and_bad_values_are_ignored() {
        let mut plugin = SimpleSmoothing::default();

        plugin.set_property("strength", "5.0");
        assert_eq!(plugin.strength, STRENGTH_MAX);
        plugin.set_property("strength", "-1");
        assert_eq!(plugin.strength, STRENGTH_MIN);

        plugin.set_property("strength", "0.25");
        plugin.set_property("strength", "null"); // the interface can send null
        plugin.set_property("strength", "\"text\"");
        plugin.set_property("strength", "not json");
        plugin.set_property("unknown_key", "0.9");
        assert_eq!(plugin.strength, 0.25);
    }

    #[test]
    fn the_defaults_match_the_manifest() {
        let plugin = SimpleSmoothing::default();
        let manifest = plugin.manifest();
        let Some(property) = manifest.properties.first() else {
            panic!("the manifest has no property");
        };
        let PropertyKind::Float {
            default, min, max, ..
        } = &property.kind
        else {
            panic!("strength should be a Float");
        };
        assert_eq!(*default, plugin.strength);
        assert!(min <= default && default <= max);
    }
}
