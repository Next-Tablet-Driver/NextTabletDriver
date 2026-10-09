//! # Plugin Filter Pipeline
//!
//! Applies active dynamic plugins sequentially to incoming tablet packets
//! during the high-frequency polling loop. Zero heap allocations are made
//! on the `process` hot path.

use crate::core::config::models::MappingConfig;
use crate::engine::plugins::{PluginInstance, PluginManager};
use ntd_plugin_api::{PluginContext, PluginPacket};

/// An active plugin registered in the pipeline with its current enable state.
pub struct ActivePluginEntry {
    /// Unique identifier for the plugin (matching [`ntd_plugin_api::PluginManifest::id`]).
    pub id: String,
    /// Live plugin instance wrapped in RAII library management.
    pub instance: PluginInstance,
    /// Whether this plugin is currently active and processing packets.
    pub enabled: bool,
}

/// The ordered collection of dynamic filter plugins executed during packet processing.
pub struct FilterPipeline {
    /// Sequential list of registered plugin instances.
    pub entries: Vec<ActivePluginEntry>,
}

impl Default for FilterPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterPipeline {
    /// Creates an empty filter pipeline.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Builds a populated pipeline from the [`PluginManager`], initializing each instance
    /// with properties and enable states from the provided [`MappingConfig`].
    #[must_use]
    pub fn from_manager(manager: &PluginManager, config: &MappingConfig) -> Self {
        let mut pipeline = Self::new();
        let manifests = manager.loaded_manifests();

        for manifest in manifests {
            if let Some(mut instance) = manager.create_instance(&manifest.id) {
                let (enabled, properties) = config
                    .plugins
                    .get(&manifest.id)
                    .map_or((false, None), |settings| {
                        (settings.enabled, Some(&settings.properties))
                    });

                if let Some(props) = properties {
                    for (key, val) in props {
                        if let Err(e) = instance.set_property(key, val) {
                            log::warn!(
                                target: "Pipeline",
                                "Failed to apply saved property '{key}' on plugin '{}': {e}",
                                manifest.id
                            );
                        }
                    }
                }

                log::info!(
                    target: "Pipeline",
                    "Registered plugin '{}' (enabled: {enabled}) in filter pipeline",
                    manifest.id
                );

                pipeline.entries.push(ActivePluginEntry {
                    id: manifest.id,
                    instance,
                    enabled,
                });
            }
        }

        pipeline
    }

    /// Adds a plugin instance directly to the pipeline.
    pub fn add_plugin(&mut self, id: String, instance: PluginInstance, enabled: bool) {
        self.entries.push(ActivePluginEntry {
            id,
            instance,
            enabled,
        });
    }

    /// Passes a packet through all enabled plugins sequentially.
    ///
    /// # Performance
    /// This method runs on the time-critical polling thread (up to 1000Hz+).
    /// Calls across the C-ABI are direct function pointer invocations with zero allocations.
    #[inline]
    pub fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext) {
        for entry in &mut self.entries {
            if entry.enabled {
                let _ = entry.instance.process(packet, context);
            }
        }
    }

    /// Updates enable states and synchronizes modified properties across all instances.
    pub fn update_config(&mut self, config: &MappingConfig) {
        for entry in &mut self.entries {
            if let Some(settings) = config.plugins.get(&entry.id) {
                entry.enabled = settings.enabled;
                for (key, val) in &settings.properties {
                    if let Err(e) = entry.instance.set_property(key, val) {
                        log::warn!(
                            target: "Pipeline",
                            "Failed to update property '{key}' for plugin '{}': {e}",
                            entry.id
                        );
                    }
                }
            } else {
                entry.enabled = false;
            }
        }
    }

    /// Clears internal state buffers across all plugin instances.
    /// Called when pen leaves proximity or tablet disconnects.
    pub fn reset(&mut self) {
        for entry in &mut self.entries {
            entry.instance.reset();
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::indexing_slicing,
    clippy::undocumented_unsafe_blocks
)]
mod tests {
    use super::*;
    use crate::core::config::models::{DynamicPluginSettings, MappingConfig};
    use crate::engine::plugins::loader::{LoadedLibrary, PluginVTable};
    use ntd_plugin_api::{PluginContext, PluginManifest, PluginPacket, PluginStatus};
    use std::collections::HashMap;
    use std::ffi::{CStr, c_char, c_void};

    struct MockState {
        process_count: u32,
        reset_count: u32,
        add_u: f32,
    }

    extern "C" fn mock_create() -> *mut c_void {
        let state = Box::new(MockState {
            process_count: 0,
            reset_count: 0,
            add_u: 0.1,
        });
        Box::into_raw(state).cast::<c_void>()
    }

    unsafe extern "C" fn mock_process(
        ptr: *mut c_void,
        packet: *mut PluginPacket,
        _context: *const PluginContext,
    ) -> PluginStatus {
        if ptr.is_null() || packet.is_null() {
            return PluginStatus::Error;
        }
        let state = unsafe { &mut *ptr.cast::<MockState>() };
        state.process_count += 1;
        unsafe {
            (*packet).u += state.add_u;
        }
        PluginStatus::Ok
    }

    unsafe extern "C" fn mock_set_property(
        ptr: *mut c_void,
        key: *const c_char,
        val: *const c_char,
    ) -> PluginStatus {
        if ptr.is_null() || key.is_null() || val.is_null() {
            return PluginStatus::Error;
        }
        let state = unsafe { &mut *ptr.cast::<MockState>() };
        let key_str = unsafe { CStr::from_ptr(key).to_string_lossy() };
        let val_str = unsafe { CStr::from_ptr(val).to_string_lossy() };

        if key_str == "add_u"
            && let Ok(v) = val_str.trim_matches('"').parse::<f32>()
        {
            state.add_u = v;
            return PluginStatus::Ok;
        }
        PluginStatus::Ok
    }

    unsafe extern "C" fn mock_reset(ptr: *mut c_void) -> PluginStatus {
        if ptr.is_null() {
            return PluginStatus::Error;
        }
        let state = unsafe { &mut *ptr.cast::<MockState>() };
        state.reset_count += 1;
        PluginStatus::Ok
    }

    unsafe extern "C" fn mock_destroy(ptr: *mut c_void) -> PluginStatus {
        if !ptr.is_null() {
            unsafe {
                drop(Box::from_raw(ptr.cast::<MockState>()));
            }
        }
        PluginStatus::Ok
    }

    fn create_mock_instance(id: &str, initial_add: f32) -> PluginInstance {
        let manifest = PluginManifest {
            id: id.to_string(),
            name: format!("Mock {id}"),
            version: "1.0.0".to_string(),
            author: "Tester".to_string(),
            description: "Mock for testing".to_string(),
            icon: None,
            properties: vec![],
        };
        let vtable = PluginVTable {
            create: mock_create,
            process: mock_process,
            set_property: mock_set_property,
            reset: mock_reset,
            destroy: mock_destroy,
        };
        let lib = LoadedLibrary::mock(manifest, vtable);
        let mut inst = PluginInstance::new(lib).expect("mock creation should succeed");
        // Set initial add_u via property
        let _ = inst.set_property("add_u", &serde_json::json!(initial_add));
        inst
    }

    #[test]
    fn test_filter_pipeline_sequential_processing_and_enabled_filter() {
        let mut pipeline = FilterPipeline::new();
        pipeline.add_plugin("plugin1".to_string(), create_mock_instance("p1", 0.1), true);
        pipeline.add_plugin(
            "plugin2".to_string(),
            create_mock_instance("p2", 0.2),
            false,
        ); // Disabled
        pipeline.add_plugin("plugin3".to_string(), create_mock_instance("p3", 0.5), true);

        let mut packet = PluginPacket {
            u: 0.0,
            v: 0.0,
            pressure: 0.5,
            tilt_x: 0,
            tilt_y: 0,
            is_down: false,
            timestamp_ns: 0,
        };
        let context = PluginContext {
            active_area_width_mm: 100.0,
            active_area_height_mm: 100.0,
            target_area_width_px: 1920.0,
            target_area_height_px: 1080.0,
        };

        pipeline.process(&mut packet, &context);

        // packet.u should be 0.0 + 0.1 (p1) + 0.5 (p3) = 0.6, skipping p2
        assert!((packet.u - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_filter_pipeline_update_config() {
        let mut pipeline = FilterPipeline::new();
        pipeline.add_plugin("p1".to_string(), create_mock_instance("p1", 0.1), true);
        pipeline.add_plugin("p2".to_string(), create_mock_instance("p2", 0.2), false);
        pipeline.add_plugin("p3".to_string(), create_mock_instance("p3", 0.3), true);

        let mut config = MappingConfig::default();
        // p1: disable
        config.plugins.insert(
            "p1".to_string(),
            DynamicPluginSettings {
                enabled: false,
                properties: HashMap::new(),
            },
        );
        // p2: enable and change property
        let mut p2_props = HashMap::new();
        p2_props.insert("add_u".to_string(), serde_json::json!(0.75));
        config.plugins.insert(
            "p2".to_string(),
            DynamicPluginSettings {
                enabled: true,
                properties: p2_props,
            },
        );
        // p3: omitted from config.plugins, so it should be disabled

        pipeline.update_config(&config);

        assert!(!pipeline.entries[0].enabled);
        assert!(pipeline.entries[1].enabled);
        assert!(!pipeline.entries[2].enabled);

        // Verify that running process now only executes p2 with the updated property
        let mut packet = PluginPacket {
            u: 0.0,
            v: 0.0,
            pressure: 0.0,
            tilt_x: 0,
            tilt_y: 0,
            is_down: false,
            timestamp_ns: 0,
        };
        let context = PluginContext {
            active_area_width_mm: 100.0,
            active_area_height_mm: 100.0,
            target_area_width_px: 1920.0,
            target_area_height_px: 1080.0,
        };

        pipeline.process(&mut packet, &context);
        assert!((packet.u - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_filter_pipeline_reset() {
        let mut pipeline = FilterPipeline::new();
        pipeline.add_plugin("p1".to_string(), create_mock_instance("p1", 0.1), true);
        pipeline.add_plugin("p2".to_string(), create_mock_instance("p2", 0.2), false);

        // Calling reset should not panic and should reset all instances
        pipeline.reset();
        assert_eq!(pipeline.entries.len(), 2);
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;

        #[test]
        fn a_default_pipeline_is_empty() {
            assert!(FilterPipeline::default().entries.is_empty());
        }

        #[test]
        fn a_manager_without_plugins_gives_an_empty_pipeline() {
            let manager = PluginManager::new();
            let pipeline = FilterPipeline::from_manager(&manager, &MappingConfig::default());
            assert!(pipeline.entries.is_empty());
        }
    }
}
