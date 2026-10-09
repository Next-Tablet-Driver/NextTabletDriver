//! # Plugin Instance
//!
//! Safe RAII wrapper around an active plugin instance created from a [`LoadedLibrary`].
//! Guarantees that the backing shared library is never unmapped while the instance exists,
//! and properly destroys the C++ or Rust instance upon Drop.

use super::error::PluginError;
use super::loader::LoadedLibrary;
use ntd_plugin_api::{PluginContext, PluginManifest, PluginPacket, PluginStatus};
use std::ffi::CString;
use std::sync::Arc;

/// A live instance of an active plugin.
///
/// Encapsulates the opaque instance pointer returned by `ntd_plugin_create`
/// and keeps the underlying library alive via `Arc<LoadedLibrary>`.
pub struct PluginInstance {
    raw_ptr: *mut std::ffi::c_void,
    lib: Arc<LoadedLibrary>,
}

// SAFETY: Each PluginInstance owns its exclusive opaque instance pointer allocated
// by the dynamic library. All method calls require `&mut self` and instances
// are passed by value across threads to the background polling loop.
unsafe impl Send for PluginInstance {}

impl PluginInstance {
    /// Creates a new instance of the plugin from the loaded library.
    ///
    /// # Errors
    /// Returns [`PluginError::InstanceCreationFailed`] if `ntd_plugin_create` returns null.
    pub fn new(lib: Arc<LoadedLibrary>) -> Result<Self, PluginError> {
        let raw_ptr = (lib.vtable.create)();
        if raw_ptr.is_null() {
            return Err(PluginError::InstanceCreationFailed);
        }
        Ok(Self { raw_ptr, lib })
    }

    /// Processes a tablet packet in place.
    ///
    /// This is the hot path invoked on every polling packet (up to 1000Hz+).
    /// Zero heap allocations are performed here.
    #[inline]
    pub fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext) -> PluginStatus {
        if self.raw_ptr.is_null() {
            return PluginStatus::Error;
        }

        // SAFETY: self.raw_ptr is non-null and was allocated by lib.vtable.create.
        // packet and context are valid references for the duration of the call.
        unsafe {
            (self.lib.vtable.process)(
                self.raw_ptr,
                std::ptr::from_mut::<PluginPacket>(packet),
                std::ptr::from_ref::<PluginContext>(context),
            )
        }
    }

    /// Updates a plugin configuration property dynamically.
    ///
    /// # Errors
    /// Returns [`PluginError`] if the property name or JSON value contains null bytes,
    /// or if the plugin rejects the property update.
    pub fn set_property(
        &mut self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), PluginError> {
        if self.raw_ptr.is_null() {
            return Err(PluginError::InstanceCreationFailed);
        }

        let k_cstr = CString::new(key)?;
        let val_str = serde_json::to_string(value)?;
        let v_cstr = CString::new(val_str)?;

        // SAFETY: self.raw_ptr is valid, k_cstr and v_cstr are valid null-terminated C strings.
        let status = unsafe {
            (self.lib.vtable.set_property)(self.raw_ptr, k_cstr.as_ptr(), v_cstr.as_ptr())
        };

        if status == PluginStatus::Ok {
            Ok(())
        } else {
            Err(PluginError::PropertyError(key.to_string()))
        }
    }

    /// Resets internal state (called when pen leaves proximity or tablet disconnects).
    pub fn reset(&mut self) {
        if !self.raw_ptr.is_null() {
            // SAFETY: self.raw_ptr is a valid instance pointer.
            unsafe {
                let _ = (self.lib.vtable.reset)(self.raw_ptr);
            }
        }
    }

    /// Returns the declarative manifest for this plugin.
    #[must_use]
    pub fn manifest(&self) -> &PluginManifest {
        &self.lib.manifest
    }

    /// Returns the unique identifier for this plugin.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.lib.manifest.id
    }
}

impl Drop for PluginInstance {
    fn drop(&mut self) {
        if !self.raw_ptr.is_null() {
            // SAFETY: self.raw_ptr was allocated by self.lib.vtable.create and has not been freed.
            // self.lib is kept alive through the Arc, so the code memory remains mapped.
            unsafe {
                let _ = (self.lib.vtable.destroy)(self.raw_ptr);
            }
            self.raw_ptr = std::ptr::null_mut();
        }
    }
}
