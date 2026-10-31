//! # Filter chain
//!
//! Runs the active filters one after the other on every packet of the polling loop. A filter is
//! anything that implements [`Filter`]: the chain does not know whether it is a dynamic plugin, a
//! built-in, or a test double. Zero heap allocations are made on the `process` hot path.

use ntd_core::config::models::MappingConfig;
use ntd_plugin_api::{PluginContext, PluginPacket};

/// Why a filter refused a property.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct FilterError(pub String);

/// One stage of the filter chain.
pub trait Filter: Send {
    /// Changes the packet in place. Runs on the time-critical polling thread (1000 Hz and more):
    /// it must not allocate or block.
    fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext);

    /// Applies a configuration property.
    ///
    /// # Errors
    /// Returns a [`FilterError`] when the filter does not accept the key or the value.
    fn set_property(&mut self, key: &str, value: &serde_json::Value) -> Result<(), FilterError>;

    /// Clears the internal state. Called when the pen leaves proximity or the tablet disconnects.
    fn reset(&mut self);
}

/// A filter registered in the chain with its current enable state.
pub struct FilterEntry {
    /// Unique identifier of the filter, the key of its settings in [`MappingConfig::plugins`].
    pub id: String,
    /// The filter itself.
    pub filter: Box<dyn Filter>,
    /// Whether the filter is currently active and processing packets.
    pub enabled: bool,
}

/// The ordered collection of filters executed during packet processing.
#[derive(Default)]
pub struct FilterPipeline {
    /// The filters, in execution order.
    pub entries: Vec<FilterEntry>,
}

impl FilterPipeline {
    /// Creates an empty chain.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Appends a filter to the end of the chain.
    pub fn add(&mut self, id: String, filter: Box<dyn Filter>, enabled: bool) {
        self.entries.push(FilterEntry {
            id,
            filter,
            enabled,
        });
    }

    /// Passes a packet through all enabled filters sequentially.
    #[inline]
    pub fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext) {
        for entry in &mut self.entries {
            if entry.enabled {
                entry.filter.process(packet, context);
            }
        }
    }

    /// Updates enable states and synchronizes modified properties across all filters. A filter
    /// that has no settings in `config` is disabled.
    pub fn update_config(&mut self, config: &MappingConfig) {
        for entry in &mut self.entries {
            if let Some(settings) = config.plugins.get(&entry.id) {
                entry.enabled = settings.enabled;
                for (key, val) in &settings.properties {
                    if let Err(e) = entry.filter.set_property(key, val) {
                        log::warn!(
                            target: "Pipeline",
                            "Failed to apply property '{key}' on filter '{}': {e}",
                            entry.id
                        );
                    }
                }
            } else {
                entry.enabled = false;
            }
        }
    }

    /// Clears the internal state of every filter.
    pub fn reset(&mut self) {
        for entry in &mut self.entries {
            entry.filter.reset();
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;
    use ntd_core::config::models::DynamicPluginSettings;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Adds a constant to `u`; `add_u` changes the constant.
    struct AddU {
        amount: f32,
        resets: Arc<AtomicU32>,
    }

    impl Filter for AddU {
        fn process(&mut self, packet: &mut PluginPacket, _context: &PluginContext) {
            packet.u += self.amount;
        }

        fn set_property(
            &mut self,
            key: &str,
            value: &serde_json::Value,
        ) -> Result<(), FilterError> {
            match (key, value.as_f64()) {
                ("add_u", Some(v)) => {
                    self.amount = v as f32;
                    Ok(())
                }
                _ => Err(FilterError(format!("unknown property {key}"))),
            }
        }

        fn reset(&mut self) {
            self.resets.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn add_u(amount: f32) -> (Box<dyn Filter>, Arc<AtomicU32>) {
        let resets = Arc::new(AtomicU32::new(0));
        let filter = AddU {
            amount,
            resets: Arc::clone(&resets),
        };
        (Box::new(filter), resets)
    }

    fn packet() -> PluginPacket {
        PluginPacket {
            u: 0.0,
            v: 0.0,
            pressure: 0.5,
            tilt_x: 0,
            tilt_y: 0,
            is_down: false,
            timestamp_ns: 0,
        }
    }

    const CONTEXT: PluginContext = PluginContext {
        active_area_width_mm: 100.0,
        active_area_height_mm: 100.0,
        target_area_width_px: 1920.0,
        target_area_height_px: 1080.0,
    };

    #[test]
    fn filters_run_in_order_and_a_disabled_one_is_skipped() {
        let mut chain = FilterPipeline::new();
        chain.add("p1".to_string(), add_u(0.1).0, true);
        chain.add("p2".to_string(), add_u(0.2).0, false);
        chain.add("p3".to_string(), add_u(0.5).0, true);

        let mut packet = packet();
        chain.process(&mut packet, &CONTEXT);

        assert!((packet.u - 0.6).abs() < 1e-6);
    }

    #[test]
    fn update_config_enables_disables_and_applies_properties() {
        let mut chain = FilterPipeline::new();
        chain.add("p1".to_string(), add_u(0.1).0, true);
        chain.add("p2".to_string(), add_u(0.2).0, false);
        chain.add("p3".to_string(), add_u(0.3).0, true);

        let mut config = MappingConfig::default();
        config.plugins.insert(
            "p1".to_string(),
            DynamicPluginSettings {
                enabled: false,
                properties: HashMap::new(),
            },
        );
        config.plugins.insert(
            "p2".to_string(),
            DynamicPluginSettings {
                enabled: true,
                properties: HashMap::from([("add_u".to_string(), serde_json::json!(0.75))]),
            },
        );
        // p3 is not in the settings: disabled.

        chain.update_config(&config);

        let enabled: Vec<bool> = chain.entries.iter().map(|e| e.enabled).collect();
        assert_eq!(enabled, [false, true, false]);
        let mut packet = packet();
        chain.process(&mut packet, &CONTEXT);
        assert!((packet.u - 0.75).abs() < 1e-6);
    }

    #[test]
    fn a_rejected_property_does_not_stop_the_update() {
        let mut chain = FilterPipeline::new();
        chain.add("p1".to_string(), add_u(0.1).0, false);
        let mut config = MappingConfig::default();
        config.plugins.insert(
            "p1".to_string(),
            DynamicPluginSettings {
                enabled: true,
                properties: HashMap::from([("nope".to_string(), serde_json::json!(1))]),
            },
        );

        chain.update_config(&config);

        assert!(chain.entries.iter().all(|e| e.enabled));
    }

    #[test]
    fn reset_reaches_every_filter_even_a_disabled_one() {
        let (first, first_resets) = add_u(0.1);
        let (second, second_resets) = add_u(0.2);
        let mut chain = FilterPipeline::new();
        chain.add("p1".to_string(), first, true);
        chain.add("p2".to_string(), second, false);

        chain.reset();

        assert_eq!(first_resets.load(Ordering::Relaxed), 1);
        assert_eq!(second_resets.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_default_chain_is_empty() {
        assert!(FilterPipeline::default().entries.is_empty());
    }
}
