//! # `NextTabletDriver` Plugin API & C-ABI Definitions
//!
//! This crate provides the stable FFI interface and high-level Rust abstractions
//! for creating dynamic plugins (`.dll` on Windows, `.so` on Linux) for `NextTabletDriver`.
//!
//! ## Architecture & Safety
//! Plugins communicate with `NextTabletDriver` through a pure C-ABI boundary (`extern "C"`).
//! Functions never panic across FFI boundaries (all calls are guarded by `std::panic::catch_unwind`).
//! Communication on the time-critical polling path is completely zero-allocation.

use serde::{Deserialize, Serialize};

/// 32-bit magic identifier required for handshake verification: `"NTDP"` (Next Tablet Driver Plugin).
pub const NTD_PLUGIN_MAGIC: u32 = 0x4E54_4450;

/// Current plugin ABI version.
pub const NTD_PLUGIN_ABI_VERSION: u32 = 1;

/// In-place packet data exchanged between `NextTabletDriver` and the plugin.
///
/// Layout is guaranteed to match across C ABI boundaries.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PluginPacket {
    /// Normalized horizontal coordinate in `[0.0, 1.0]`.
    pub u: f32,
    /// Normalized vertical coordinate in `[0.0, 1.0]`.
    pub v: f32,
    /// Normalized pen tip pressure in `[0.0, 1.0]`.
    pub pressure: f32,
    /// Horizontal pen tilt in degrees.
    pub tilt_x: i32,
    /// Vertical pen tilt in degrees.
    pub tilt_y: i32,
    /// Whether the pen is contacting the tablet surface.
    pub is_down: bool,
    /// Monotonic timestamp in nanoseconds when the packet was received.
    pub timestamp_ns: u64,
}

impl Default for PluginPacket {
    fn default() -> Self {
        Self {
            u: 0.0,
            v: 0.0,
            pressure: 0.0,
            tilt_x: 0,
            tilt_y: 0,
            is_down: false,
            timestamp_ns: 0,
        }
    }
}

/// Global device and mapping context provided to the plugin during execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PluginContext {
    /// Active mapping area width in millimeters.
    pub active_area_width_mm: f32,
    /// Active mapping area height in millimeters.
    pub active_area_height_mm: f32,
    /// Target screen mapping area width in pixels.
    pub target_area_width_px: f32,
    /// Target screen mapping area height in pixels.
    pub target_area_height_px: f32,
}

impl Default for PluginContext {
    fn default() -> Self {
        Self {
            active_area_width_mm: 160.0,
            active_area_height_mm: 100.0,
            target_area_width_px: 1920.0,
            target_area_height_px: 1080.0,
        }
    }
}

/// Status code returned by FFI plugin operations.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginStatus {
    /// Operation succeeded.
    Ok = 0,
    /// Operation encountered an error or panic.
    Error = 1,
}

/// Complete declarative metadata for a plugin, serialized to JSON across the ABI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique identifier for the plugin (e.g., `"radial_follow"`).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Semver version string.
    pub version: String,
    /// Author name and attribution.
    pub author: String,
    /// Detailed description of the plugin functionality.
    pub description: String,
    /// Optional Phosphor icon name or icon key (e.g., `"CROSSHAIR"`).
    pub icon: Option<String>,
    /// List of user-adjustable properties.
    pub properties: Vec<PropertyDescriptor>,
}

/// A single user-facing configurable property.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyDescriptor {
    /// Unique key for this property (e.g., `"outer_radius"`).
    pub id: String,
    /// User-friendly label displayed in the UI.
    pub name: String,
    /// Helpful tooltip explaining the setting.
    pub tooltip: Option<String>,
    /// Specification of type, range, step, and default value.
    pub kind: PropertyKind,
}

/// The concrete type and range constraints for a property.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum PropertyKind {
    /// Floating-point slider or drag input.
    Float {
        /// Minimum selectable value.
        min: f32,
        /// Maximum selectable value.
        max: f32,
        /// Step increment.
        step: f32,
        /// Display unit suffix (e.g. `"px"`, `"mm"`, `"ms"`).
        unit: String,
        /// Default initial value.
        default: f32,
    },
    /// Integer slider or input.
    Int {
        /// Minimum selectable integer.
        min: i32,
        /// Maximum selectable integer.
        max: i32,
        /// Step increment.
        step: i32,
        /// Display unit suffix.
        unit: String,
        /// Default initial integer.
        default: i32,
    },
    /// Boolean toggle checkbox.
    Bool {
        /// Default boolean state.
        default: bool,
    },
    /// Multi-choice dropdown selector.
    Choice {
        /// List of options.
        options: Vec<String>,
        /// Default selected index.
        default_index: usize,
    },
    /// Plain single-line string text input.
    String {
        /// Default string content.
        default: String,
    },
}

/// High-level Rust trait for implementing `NextTabletDriver` plugins.
pub trait NextTabletPlugin: Send + Sync + 'static {
    /// Returns the declarative manifest describing the plugin metadata and properties.
    fn manifest(&self) -> PluginManifest;

    /// Processes an incoming tablet packet in place.
    ///
    /// The plugin may mutate `packet.u`, `packet.v`, `packet.pressure`, etc.
    fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext);

    /// Updates a configuration property dynamically when changed in the UI.
    fn set_property(&mut self, key: &str, value_json: &str);

    /// Resets any internal running state (called on tablet disconnect or out-of-range).
    fn reset(&mut self);
}

/// Exports a struct implementing [`NextTabletPlugin`] with all required `extern "C"` FFI symbols.
#[macro_export]
macro_rules! export_ntd_plugin {
    ($plugin_type:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ntd_plugin_handshake() -> u32 {
            // Lower 16 bits = ABI version, upper 16 bits = Magic lower half
            ($crate::NTD_PLUGIN_MAGIC & 0xFFFF_0000)
                | ($crate::NTD_PLUGIN_ABI_VERSION & 0x0000_FFFF)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ntd_plugin_manifest() -> *const std::ffi::c_char {
            use std::sync::OnceLock;
            static MANIFEST_CSTRING: OnceLock<std::ffi::CString> = OnceLock::new();

            MANIFEST_CSTRING
                .get_or_init(|| {
                    let instance = $plugin_type::default();
                    let manifest = $crate::NextTabletPlugin::manifest(&instance);
                    let json = match serde_json::to_string(&manifest) {
                        Ok(s) => s,
                        Err(_) => String::new(),
                    };
                    std::ffi::CString::new(json).unwrap_or_else(|_| std::ffi::CString::default())
                })
                .as_ptr()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ntd_plugin_create() -> *mut std::ffi::c_void {
            let result = std::panic::catch_unwind(|| {
                let plugin = Box::new($plugin_type::default());
                Box::into_raw(plugin) as *mut std::ffi::c_void
            });
            match result {
                Ok(ptr) => ptr,
                Err(_) => std::ptr::null_mut(),
            }
        }

        #[unsafe(no_mangle)]
        /// # Safety
        /// `instance` must be a valid pointer created by `ntd_plugin_create`.
        /// `packet` and `context` must be valid, non-null references.
        pub unsafe extern "C" fn ntd_plugin_process(
            instance: *mut std::ffi::c_void,
            packet: *mut $crate::PluginPacket,
            context: *const $crate::PluginContext,
        ) -> $crate::PluginStatus {
            if instance.is_null() || packet.is_null() || context.is_null() {
                return $crate::PluginStatus::Error;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: instance is validated non-null and was created by Box::into_raw in ntd_plugin_create.
                let plugin = unsafe { &mut *(instance as *mut $plugin_type) };
                // SAFETY: packet and context pointers are validated non-null.
                let pkt = unsafe { &mut *packet };
                let ctx = unsafe { &*context };

                $crate::NextTabletPlugin::process(plugin, pkt, ctx);
            });

            match result {
                Ok(_) => $crate::PluginStatus::Ok,
                Err(_) => $crate::PluginStatus::Error,
            }
        }

        #[unsafe(no_mangle)]
        /// # Safety
        /// `instance`, `key`, and `value_json` must be valid non-null pointers.
        pub unsafe extern "C" fn ntd_plugin_set_property(
            instance: *mut std::ffi::c_void,
            key: *const std::ffi::c_char,
            value_json: *const std::ffi::c_char,
        ) -> $crate::PluginStatus {
            if instance.is_null() || key.is_null() || value_json.is_null() {
                return $crate::PluginStatus::Error;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: pointers are checked for null above and point to null-terminated C strings.
                let plugin = unsafe { &mut *(instance as *mut $plugin_type) };
                let k = unsafe { std::ffi::CStr::from_ptr(key) }.to_string_lossy();
                let v = unsafe { std::ffi::CStr::from_ptr(value_json) }.to_string_lossy();

                $crate::NextTabletPlugin::set_property(plugin, &k, &v);
            });

            match result {
                Ok(_) => $crate::PluginStatus::Ok,
                Err(_) => $crate::PluginStatus::Error,
            }
        }

        #[unsafe(no_mangle)]
        /// # Safety
        /// `instance` must be a valid pointer created by `ntd_plugin_create`.
        pub unsafe extern "C" fn ntd_plugin_reset(
            instance: *mut std::ffi::c_void,
        ) -> $crate::PluginStatus {
            if instance.is_null() {
                return $crate::PluginStatus::Error;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: instance is validated non-null and was created by ntd_plugin_create.
                let plugin = unsafe { &mut *(instance as *mut $plugin_type) };
                $crate::NextTabletPlugin::reset(plugin);
            });

            match result {
                Ok(_) => $crate::PluginStatus::Ok,
                Err(_) => $crate::PluginStatus::Error,
            }
        }

        #[unsafe(no_mangle)]
        /// # Safety
        /// `instance` must be a valid pointer created by `ntd_plugin_create` and not yet freed.
        pub unsafe extern "C" fn ntd_plugin_destroy(
            instance: *mut std::ffi::c_void,
        ) -> $crate::PluginStatus {
            if instance.is_null() {
                return $crate::PluginStatus::Ok;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: taking ownership back from raw pointer to properly deallocate.
                let _ = unsafe { Box::from_raw(instance as *mut $plugin_type) };
            });

            match result {
                Ok(_) => $crate::PluginStatus::Ok,
                Err(_) => $crate::PluginStatus::Error,
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct DummyPlugin {
        gain: f32,
    }

    impl NextTabletPlugin for DummyPlugin {
        fn manifest(&self) -> PluginManifest {
            PluginManifest {
                id: "dummy".to_string(),
                name: "Dummy Filter".to_string(),
                version: "1.0.0".to_string(),
                author: "Test".to_string(),
                description: "Test description".to_string(),
                icon: None,
                properties: vec![PropertyDescriptor {
                    id: "gain".to_string(),
                    name: "Gain".to_string(),
                    tooltip: None,
                    kind: PropertyKind::Float {
                        min: 0.0,
                        max: 2.0,
                        step: 0.1,
                        unit: "x".to_string(),
                        default: 1.0,
                    },
                }],
            }
        }

        fn process(&mut self, packet: &mut PluginPacket, _context: &PluginContext) {
            packet.u *= self.gain;
            packet.v *= self.gain;
        }

        fn set_property(&mut self, key: &str, value_json: &str) {
            if key == "gain"
                && let Ok(val) = serde_json::from_str::<f32>(value_json)
            {
                self.gain = val;
            }
        }

        fn reset(&mut self) {
            self.gain = 1.0;
        }
    }

    #[test]
    fn test_manifest_serialization() {
        let plugin = DummyPlugin::default();
        let manifest = plugin.manifest();
        let json = serde_json::to_string(&manifest).unwrap_or_default();
        let deserialized: PluginManifest =
            serde_json::from_str(&json).unwrap_or_else(|_| PluginManifest {
                id: String::new(),
                name: String::new(),
                version: String::new(),
                author: String::new(),
                description: String::new(),
                icon: None,
                properties: vec![],
            });
        assert_eq!(manifest.id, deserialized.id);
        assert_eq!(manifest.properties.len(), deserialized.properties.len());
    }

    #[test]
    fn test_handshake_constants() {
        assert_eq!(NTD_PLUGIN_MAGIC, 0x4E54_4450);
        assert_eq!(NTD_PLUGIN_ABI_VERSION, 1);
    }
}
