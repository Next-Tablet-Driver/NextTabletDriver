//! # Plugin Loader
//!
//! Loads dynamic libraries, executes the ABI handshake, parses the plugin manifest,
//! and extracts function pointers into a thread-safe `LoadedLibrary`.

use super::error::PluginError;
use ntd_plugin_api::{
    NTD_PLUGIN_ABI_VERSION, NTD_PLUGIN_MAGIC, PluginContext, PluginManifest, PluginPacket,
    PluginStatus,
};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) type HandshakeFn = extern "C" fn() -> u32;
pub(crate) type ManifestFn = extern "C" fn() -> *const c_char;
pub(crate) type CreateFn = extern "C" fn() -> *mut std::ffi::c_void;
pub(crate) type ProcessFn = unsafe extern "C" fn(
    *mut std::ffi::c_void,
    *mut PluginPacket,
    *const PluginContext,
) -> PluginStatus;
pub(crate) type SetPropertyFn =
    unsafe extern "C" fn(*mut std::ffi::c_void, *const c_char, *const c_char) -> PluginStatus;
pub(crate) type ResetFn = unsafe extern "C" fn(*mut std::ffi::c_void) -> PluginStatus;
pub(crate) type DestroyFn = unsafe extern "C" fn(*mut std::ffi::c_void) -> PluginStatus;

/// Function pointers table extracted from a loaded plugin library.
#[derive(Clone, Copy)]
pub(crate) struct PluginVTable {
    pub create: CreateFn,
    pub process: ProcessFn,
    pub set_property: SetPropertyFn,
    pub reset: ResetFn,
    pub destroy: DestroyFn,
}

/// An open shared library handle along with its verified manifest and call table.
pub struct LoadedLibrary {
    /// Kept alive to prevent OS code memory from being unmapped.
    _lib: Option<libloading::Library>,
    /// Declarative metadata and property descriptors.
    pub manifest: PluginManifest,
    /// Function pointers table.
    pub(crate) vtable: PluginVTable,
    /// Path on disk from which the library was loaded.
    pub path: PathBuf,
}

#[cfg(test)]
impl LoadedLibrary {
    #[must_use]
    pub(crate) fn mock(manifest: PluginManifest, vtable: PluginVTable) -> Arc<Self> {
        Arc::new(Self {
            _lib: None,
            manifest,
            vtable,
            path: PathBuf::from("mock_plugin"),
        })
    }
}

/// Utility for safely loading and verifying plugin dynamic libraries.
pub struct PluginLoader;

impl PluginLoader {
    /// Loads a plugin dynamic library from the specified file path, performing
    /// ABI version negotiation and manifest verification.
    ///
    /// # Errors
    /// Returns [`PluginError`] if the library cannot be loaded, lacks required FFI symbols,
    /// fails the magic/version handshake, or produces an invalid manifest.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Arc<LoadedLibrary>, PluginError> {
        let path_ref = path.as_ref();

        // SAFETY: Loading a dynamic library is unsafe because executing library initialization
        // code may run arbitrary logic. This is the explicit purpose of loading external plugins.
        let lib = unsafe { libloading::Library::new(path_ref) }?;

        // SAFETY: Symbol lookup of a known C function signature exported by export_ntd_plugin!
        let handshake_sym: libloading::Symbol<HandshakeFn> =
            unsafe { lib.get(b"ntd_plugin_handshake\0") }
                .map_err(|_| PluginError::MissingSymbol("ntd_plugin_handshake"))?;

        let handshake_code = handshake_sym();
        let magic = handshake_code & 0xFFFF_0000;
        let version = handshake_code & 0x0000_FFFF;

        let expected_magic = NTD_PLUGIN_MAGIC & 0xFFFF_0000;
        if magic != expected_magic {
            return Err(PluginError::InvalidMagic {
                expected: expected_magic,
                found: magic,
            });
        }

        if version != NTD_PLUGIN_ABI_VERSION {
            return Err(PluginError::IncompatibleAbiVersion {
                expected: NTD_PLUGIN_ABI_VERSION,
                found: version,
            });
        }

        // SAFETY: Symbol lookup of the manifest generation function.
        let manifest_sym: libloading::Symbol<ManifestFn> =
            unsafe { lib.get(b"ntd_plugin_manifest\0") }
                .map_err(|_| PluginError::MissingSymbol("ntd_plugin_manifest"))?;

        let manifest_ptr = manifest_sym();
        if manifest_ptr.is_null() {
            return Err(PluginError::ManifestError(
                "ntd_plugin_manifest returned a null pointer".to_string(),
            ));
        }

        // SAFETY: manifest_ptr is checked non-null and points to a null-terminated C string in static memory.
        let manifest_cstr = unsafe { CStr::from_ptr(manifest_ptr) };
        let manifest_json = manifest_cstr.to_str().map_err(|e| {
            PluginError::ManifestError(format!("Manifest contains invalid UTF-8: {e}"))
        })?;

        let manifest: PluginManifest = serde_json::from_str(manifest_json)
            .map_err(|e| PluginError::ManifestError(format!("Manifest JSON parse error: {e}")))?;

        // SAFETY: Symbol lookup for ntd_plugin_create matching CreateFn signature.
        let create: libloading::Symbol<CreateFn> = unsafe { lib.get(b"ntd_plugin_create\0") }
            .map_err(|_| PluginError::MissingSymbol("ntd_plugin_create"))?;
        // SAFETY: Symbol lookup for ntd_plugin_process matching ProcessFn signature.
        let process: libloading::Symbol<ProcessFn> = unsafe { lib.get(b"ntd_plugin_process\0") }
            .map_err(|_| PluginError::MissingSymbol("ntd_plugin_process"))?;
        // SAFETY: Symbol lookup for ntd_plugin_set_property matching SetPropertyFn signature.
        let set_property: libloading::Symbol<SetPropertyFn> =
            unsafe { lib.get(b"ntd_plugin_set_property\0") }
                .map_err(|_| PluginError::MissingSymbol("ntd_plugin_set_property"))?;
        // SAFETY: Symbol lookup for ntd_plugin_reset matching ResetFn signature.
        let reset: libloading::Symbol<ResetFn> = unsafe { lib.get(b"ntd_plugin_reset\0") }
            .map_err(|_| PluginError::MissingSymbol("ntd_plugin_reset"))?;
        // SAFETY: Symbol lookup for ntd_plugin_destroy matching DestroyFn signature.
        let destroy: libloading::Symbol<DestroyFn> = unsafe { lib.get(b"ntd_plugin_destroy\0") }
            .map_err(|_| PluginError::MissingSymbol("ntd_plugin_destroy"))?;

        let vtable = PluginVTable {
            create: *create,
            process: *process,
            set_property: *set_property,
            reset: *reset,
            destroy: *destroy,
        };

        log::info!(
            target: "Plugins",
            "Loaded plugin '{}' v{} by {} from {}",
            manifest.name,
            manifest.version,
            manifest.author,
            path_ref.display()
        );

        Ok(Arc::new(LoadedLibrary {
            _lib: Some(lib),
            manifest,
            vtable,
            path: path_ref.to_path_buf(),
        }))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::engine::plugins::instance::PluginInstance;

    #[test]
    fn test_load_radial_follow_plugin_if_present() {
        #[cfg(target_os = "windows")]
        let dev_path = PathBuf::from("plugins/radial_follow/target/debug/radial_follow.dll");
        #[cfg(not(target_os = "windows"))]
        let dev_path = PathBuf::from("plugins/radial_follow/target/debug/libradial_follow.so");

        let dll_path = if dev_path.exists() {
            dev_path
        } else {
            PathBuf::from("plugins/radial_follow.dll")
        };
        if !dll_path.exists() {
            return;
        }

        let loaded = PluginLoader::load(&dll_path).expect("failed to load radial_follow plugin");
        assert_eq!(loaded.manifest.id, "radial_follow");
        assert_eq!(loaded.manifest.name, "Radial Follow Smoothing");
        assert!(loaded.manifest.author.contains("AbstractQbit"));
        assert_eq!(loaded.manifest.properties.len(), 6);

        let mut instance = PluginInstance::new(Arc::clone(&loaded)).expect("failed to instantiate");
        let mut packet = PluginPacket {
            u: 0.5,
            v: 0.5,
            pressure: 0.8,
            tilt_x: 0,
            tilt_y: 0,
            is_down: true,
            timestamp_ns: 1000,
        };
        let context = PluginContext::default();

        let status = instance.process(&mut packet, &context);
        assert_eq!(status, PluginStatus::Ok);
        assert!((packet.u - 0.5).abs() < f32::EPSILON);
        assert!((packet.v - 0.5).abs() < f32::EPSILON);

        packet.u = 0.55;
        packet.v = 0.55;
        let status = instance.process(&mut packet, &context);
        assert_eq!(status, PluginStatus::Ok);
        assert!(packet.u > 0.5 && packet.u <= 0.55);

        let prop_res = instance.set_property("outer_radius", &serde_json::json!(10.0));
        assert!(prop_res.is_ok());

        instance.reset();
    }
}
