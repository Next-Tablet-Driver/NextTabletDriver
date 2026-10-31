//! # Plugin filter
//!
//! Adapts a loaded plugin ([`PluginInstance`]) to the [`Filter`] trait of `ntd-pipeline`, so the
//! pipeline runs plugins without knowing they exist.

use super::instance::PluginInstance;
use ntd_pipeline::{Filter, FilterError};
use ntd_plugin_api::{PluginContext, PluginPacket};

/// A plugin instance running as a filter of the chain.
pub struct PluginFilter(PluginInstance);

impl PluginFilter {
    /// Wraps `instance`.
    #[must_use]
    pub const fn new(instance: PluginInstance) -> Self {
        Self(instance)
    }
}

impl Filter for PluginFilter {
    #[inline]
    fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext) {
        // A plugin that reports an error leaves the packet as it is: the chain goes on.
        let _ = self.0.process(packet, context);
    }

    fn set_property(&mut self, key: &str, value: &serde_json::Value) -> Result<(), FilterError> {
        self.0
            .set_property(key, value)
            .map_err(|e| FilterError(e.to_string()))
    }

    fn reset(&mut self) {
        self.0.reset();
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::undocumented_unsafe_blocks,
    unsafe_code,
    reason = "the tests drive a mock plugin through the C ABI and compare floats it returns \
        unchanged"
)]
mod tests {
    use super::*;
    use crate::engine::plugins::PluginManager;
    use crate::engine::plugins::loader::{LoadedLibrary, PluginVTable};
    use ntd_core::config::models::{DynamicPluginSettings, MappingConfig};
    use ntd_pipeline::FilterPipeline;
    use ntd_plugin_api::{PluginManifest, PluginStatus};
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
    fn a_plugin_runs_as_a_filter_with_its_properties_and_resets() {
        let mut chain = FilterPipeline::new();
        chain.add(
            "p1".to_string(),
            Box::new(PluginFilter::new(create_mock_instance("p1", 0.1))),
            true,
        );
        chain.add(
            "p2".to_string(),
            Box::new(PluginFilter::new(create_mock_instance("p2", 0.2))),
            false,
        );
        let mut config = MappingConfig::default();
        config.plugins.insert(
            "p1".to_string(),
            DynamicPluginSettings {
                enabled: true,
                properties: HashMap::from([("add_u".to_string(), serde_json::json!(0.75))]),
            },
        );

        chain.update_config(&config);
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
        chain.process(&mut packet, &context);
        chain.reset();

        assert!((packet.u - 0.75).abs() < 1e-6);
        assert_eq!(chain.entries.len(), 2);
    }

    #[test]
    fn a_manager_without_plugins_gives_an_empty_pipeline() {
        let manager = PluginManager::empty();
        let pipeline = manager.create_pipeline(&MappingConfig::default());
        assert!(pipeline.entries.is_empty());
    }
}
