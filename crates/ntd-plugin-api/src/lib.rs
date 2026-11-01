//! # `NextTabletDriver` Plugin API & C-ABI Definitions
//!
//! This crate provides the stable FFI interface and high-level Rust abstractions
//! for creating dynamic plugins (`.dll` on Windows, `.so` on Linux) for `NextTabletDriver`.
//!
//! ## Architecture & Safety
//! Plugins communicate with `NextTabletDriver` through a pure C-ABI boundary (`extern "C"`).
//! Functions never panic across FFI boundaries (all calls are guarded by
//! `std::panic::catch_unwind`).
//! Communication on the time-critical polling path is completely zero-allocation.
//!
//! This is **ABI v3**. Every type that crosses the boundary has a fixed width (`u8`, `i32`, `u32`,
//! `u64`, `f32`), the manifest names the target the plugin was built for, and a library built for
//! an older ABI is refused by the host. See `docs/plugin-migration-v3.md` to port a v2 plugin.

use serde::{Deserialize, Serialize};

#[doc(hidden)]
pub use serde_json;

/// 32-bit magic identifier required for handshake verification: `"NTDP"` (Next Tablet Driver
/// Plugin).
pub const NTD_PLUGIN_MAGIC: u32 = 0x4E54_4450;

/// Current plugin ABI version.
///
/// Within a major version the ABI never breaks; the next incompatible change is ABI v4.
pub const NTD_PLUGIN_ABI_VERSION: u32 = 3;

/// Target triple this crate (and so the plugin that exports through [`export_ntd_plugin!`]) is
/// compiled for, for the targets the driver supports; `"unknown"` for any other.
///
/// A native library only runs on the platform it was built for: the host compares this with its
/// own value and refuses a plugin built for another target with a clear message, before the OS
/// loader gives a cryptic one.
pub const TARGET_TRIPLE: &str = target_triple();

const fn target_triple() -> &'static str {
    if cfg!(all(
        target_arch = "x86_64",
        target_os = "windows",
        target_env = "msvc"
    )) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(
        target_arch = "aarch64",
        target_os = "windows",
        target_env = "msvc"
    )) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(
        target_arch = "x86_64",
        target_os = "linux",
        target_env = "gnu"
    )) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(
        target_arch = "aarch64",
        target_os = "linux",
        target_env = "gnu"
    )) {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        "aarch64-apple-darwin"
    } else {
        "unknown"
    }
}

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
    /// `1` while the pen contacts the tablet surface, `0` otherwise. A `u8` and not a `bool`
    /// because a plugin that writes any other byte into a `bool` is undefined behaviour.
    pub is_down: u8,
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
            is_down: 0,
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

/// Raw status of a successful FFI call.
pub const NTD_STATUS_OK: i32 = 0;
/// Raw status of a failed FFI call (an error, or a panic caught inside the plugin).
pub const NTD_STATUS_ERROR: i32 = 1;

/// Status of an FFI plugin operation.
///
/// Across the ABI the status is a plain `i32` ([`NTD_STATUS_OK`], [`NTD_STATUS_ERROR`]): a foreign
/// function returning a value that is not a variant of a Rust `enum` would be undefined behaviour.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginStatus {
    /// Operation succeeded.
    Ok = NTD_STATUS_OK,
    /// Operation encountered an error or panic.
    Error = NTD_STATUS_ERROR,
}

impl PluginStatus {
    /// Reads a raw status: `0` is success, every other value is an error.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        if raw == NTD_STATUS_OK {
            Self::Ok
        } else {
            Self::Error
        }
    }

    /// The raw status to return across the ABI.
    #[must_use]
    pub const fn as_raw(self) -> i32 {
        self as i32
    }
}

/// Complete declarative metadata for a plugin, serialized to JSON across the ABI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PluginManifest {
    /// Unique identifier for the plugin (e.g., `"radial_follow"`).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Semver version string.
    pub version: String,
    /// Target triple the library was built for (see [`TARGET_TRIPLE`]). Filled in by
    /// [`export_ntd_plugin!`]; whatever [`NextTabletPlugin::manifest`] puts here is replaced.
    #[serde(default)]
    pub target: String,
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
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
        #[cfg_attr(feature = "specta", specta(type = u32))]
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
        /// Returns the plugin magic number and the ABI version, so the host can refuse a library
        /// built for another interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn ntd_plugin_handshake() -> u32 {
            // Lower 16 bits = ABI version, upper 16 bits = Magic lower half
            ($crate::NTD_PLUGIN_MAGIC & 0xFFFF_0000)
                | ($crate::NTD_PLUGIN_ABI_VERSION & 0x0000_FFFF)
        }

        /// Returns the plugin manifest as a NUL-terminated JSON string that stays valid for as long
        /// as the library is loaded.
        #[unsafe(no_mangle)]
        pub extern "C" fn ntd_plugin_manifest() -> *const std::ffi::c_char {
            use std::sync::OnceLock;
            static MANIFEST_CSTRING: OnceLock<std::ffi::CString> = OnceLock::new();

            MANIFEST_CSTRING
                .get_or_init(|| {
                    // A panic must not unwind out of an `extern "C"` function.
                    let json = std::panic::catch_unwind(|| {
                        let instance = $plugin_type::default();
                        let mut manifest = $crate::NextTabletPlugin::manifest(&instance);
                        manifest.target = $crate::TARGET_TRIPLE.to_string();
                        $crate::serde_json::to_string(&manifest).unwrap_or_default()
                    })
                    .unwrap_or_default();
                    std::ffi::CString::new(json).unwrap_or_default()
                })
                .as_ptr()
        }

        /// Creates a plugin instance and returns an owning pointer to it, or null if creating it
        /// panicked.
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
        ) -> i32 {
            if instance.is_null() || packet.is_null() || context.is_null() {
                return $crate::NTD_STATUS_ERROR;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: instance is validated non-null and was created by Box::into_raw in
                // ntd_plugin_create.
                let plugin = unsafe { &mut *(instance as *mut $plugin_type) };
                // SAFETY: packet and context pointers are validated non-null.
                let pkt = unsafe { &mut *packet };
                let ctx = unsafe { &*context };

                $crate::NextTabletPlugin::process(plugin, pkt, ctx);
            });

            match result {
                Ok(_) => $crate::NTD_STATUS_OK,
                Err(_) => $crate::NTD_STATUS_ERROR,
            }
        }

        #[unsafe(no_mangle)]
        /// # Safety
        /// `instance`, `key`, and `value_json` must be valid non-null pointers.
        pub unsafe extern "C" fn ntd_plugin_set_property(
            instance: *mut std::ffi::c_void,
            key: *const std::ffi::c_char,
            value_json: *const std::ffi::c_char,
        ) -> i32 {
            if instance.is_null() || key.is_null() || value_json.is_null() {
                return $crate::NTD_STATUS_ERROR;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: pointers are checked for null above and point to null-terminated C
                // strings.
                let plugin = unsafe { &mut *(instance as *mut $plugin_type) };
                let k = unsafe { std::ffi::CStr::from_ptr(key) }.to_string_lossy();
                let v = unsafe { std::ffi::CStr::from_ptr(value_json) }.to_string_lossy();

                $crate::NextTabletPlugin::set_property(plugin, &k, &v);
            });

            match result {
                Ok(_) => $crate::NTD_STATUS_OK,
                Err(_) => $crate::NTD_STATUS_ERROR,
            }
        }

        #[unsafe(no_mangle)]
        /// # Safety
        /// `instance` must be a valid pointer created by `ntd_plugin_create`.
        pub unsafe extern "C" fn ntd_plugin_reset(instance: *mut std::ffi::c_void) -> i32 {
            if instance.is_null() {
                return $crate::NTD_STATUS_ERROR;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: instance is validated non-null and was created by ntd_plugin_create.
                let plugin = unsafe { &mut *(instance as *mut $plugin_type) };
                $crate::NextTabletPlugin::reset(plugin);
            });

            match result {
                Ok(_) => $crate::NTD_STATUS_OK,
                Err(_) => $crate::NTD_STATUS_ERROR,
            }
        }

        #[unsafe(no_mangle)]
        /// # Safety
        /// `instance` must be a valid pointer created by `ntd_plugin_create` and not yet freed.
        pub unsafe extern "C" fn ntd_plugin_destroy(instance: *mut std::ffi::c_void) -> i32 {
            if instance.is_null() {
                return $crate::NTD_STATUS_OK;
            }

            let result = std::panic::catch_unwind(|| {
                // SAFETY: taking ownership back from raw pointer to properly deallocate.
                let _ = unsafe { Box::from_raw(instance as *mut $plugin_type) };
            });

            match result {
                Ok(_) => $crate::NTD_STATUS_OK,
                Err(_) => $crate::NTD_STATUS_ERROR,
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
                target: String::new(),
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
                target: String::new(),
                author: String::new(),
                description: String::new(),
                icon: None,
                properties: vec![],
            });
        assert_eq!(manifest.id, deserialized.id);
        assert_eq!(manifest.properties.len(), deserialized.properties.len());
    }

    #[test]
    fn the_abi_types_have_a_fixed_layout() {
        assert_eq!(std::mem::size_of::<PluginPacket>(), 32);
        assert_eq!(std::mem::align_of::<PluginPacket>(), 8);
        assert_eq!(std::mem::size_of::<PluginContext>(), 16);
        assert_eq!(std::mem::size_of::<PluginStatus>(), 4);
    }

    #[test]
    fn a_raw_status_other_than_zero_is_an_error() {
        assert_eq!(PluginStatus::from_raw(0), PluginStatus::Ok);
        assert_eq!(PluginStatus::from_raw(1), PluginStatus::Error);
        assert_eq!(PluginStatus::from_raw(-7), PluginStatus::Error);
        assert_eq!(PluginStatus::Ok.as_raw(), NTD_STATUS_OK);
        assert_eq!(PluginStatus::Error.as_raw(), NTD_STATUS_ERROR);
    }

    #[test]
    fn the_target_triple_is_known_on_a_supported_platform() {
        assert!(!TARGET_TRIPLE.is_empty());
        if cfg!(all(
            target_arch = "x86_64",
            target_os = "windows",
            target_env = "msvc"
        )) {
            assert_eq!(TARGET_TRIPLE, "x86_64-pc-windows-msvc");
        }
    }

    #[test]
    fn test_handshake_constants() {
        assert_eq!(NTD_PLUGIN_MAGIC, 0x4E54_4450);
        assert_eq!(NTD_PLUGIN_ABI_VERSION, 3);
    }
}
