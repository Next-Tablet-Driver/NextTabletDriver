//! # Plugin System Errors
//!
//! Strongly-typed errors encountered during dynamic library loading,
//! ABI handshake negotiation, property synchronization, execution, trust decisions
//! and plugin removal.

use std::path::PathBuf;

/// Errors produced during plugin loading, validation, or invocation.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// An I/O error occurred while inspecting plugin files or directories.
    #[error("Plugin I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A dynamic library loading error occurred via `libloading`.
    #[error("Dynamic library load error: {0}")]
    LibraryLoading(#[from] libloading::Error),
    /// The plugin binary did not present the expected magic identifier.
    #[error("Invalid plugin magic identifier: expected 0x{expected:08X}, found 0x{found:08X}")]
    InvalidMagic {
        /// The expected magic constant.
        expected: u32,
        /// The magic constant retrieved from the plugin handshake.
        found: u32,
    },
    /// The plugin binary targets an incompatible ABI version.
    #[error("Incompatible plugin ABI version: expected {expected}, found {found}")]
    IncompatibleAbiVersion {
        /// The expected host ABI version.
        expected: u32,
        /// The ABI version requested by the plugin.
        found: u32,
    },
    /// A required FFI symbol was not exported by the plugin library.
    #[error("Missing required plugin FFI symbol: '{0}'")]
    MissingSymbol(&'static str),
    /// The plugin manifest could not be read or deserialized.
    #[error("Failed to parse plugin manifest: {0}")]
    ManifestError(String),
    /// Instantiation of the plugin instance via `ntd_plugin_create` failed.
    #[error("Plugin instance creation returned a null pointer")]
    InstanceCreationFailed,
    /// An error occurred while setting a plugin configuration property.
    #[error("Failed to set plugin property '{0}'")]
    PropertyError(String),
    /// JSON serialization or deserialization failed.
    #[error("Plugin serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// String conversion to null-terminated C string failed due to interior nulls.
    #[error("CString conversion error: {0}")]
    CString(#[from] std::ffi::NulError),
}

/// Errors produced while approving or revoking a plugin library.
#[derive(Debug, thiserror::Error)]
pub enum TrustError {
    /// The library is not waiting for approval, for instance because the file changed since
    /// it was listed.
    #[error("This plugin is no longer pending approval (the file changed?)")]
    NotPending,
    /// The library could not be read, or the trust store could not be written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Errors produced while removing a plugin library from disk.
#[derive(Debug, thiserror::Error)]
pub enum DeletePluginError {
    /// No loaded plugin has this id.
    #[error("Plugin '{id}' is not loaded")]
    NotLoaded {
        /// The id that was asked for.
        id: String,
    },
    /// The library file could not be removed.
    #[error("Failed to delete plugin file at {}: {source}", path.display())]
    Remove {
        /// The library file.
        path: PathBuf,
        /// Why it could not be removed.
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn messages_name_what_went_wrong() {
        let cases = [
            (
                PluginError::InvalidMagic {
                    expected: 0xDEAD_BEEF,
                    found: 0x1234,
                },
                "Invalid plugin magic identifier: expected 0xDEADBEEF, found 0x00001234",
            ),
            (
                PluginError::IncompatibleAbiVersion {
                    expected: 3,
                    found: 2,
                },
                "Incompatible plugin ABI version: expected 3, found 2",
            ),
            (
                PluginError::MissingSymbol("ntd_plugin_create"),
                "Missing required plugin FFI symbol: 'ntd_plugin_create'",
            ),
            (
                PluginError::ManifestError("bad json".into()),
                "Failed to parse plugin manifest: bad json",
            ),
            (
                PluginError::InstanceCreationFailed,
                "Plugin instance creation returned a null pointer",
            ),
            (
                PluginError::PropertyError("gain".into()),
                "Failed to set plugin property 'gain'",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
            // Errors without an underlying cause do not report a source.
            assert!(error.source().is_none());
        }
    }

    #[test]
    fn io_errors_convert_and_keep_their_source() {
        let error = PluginError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(matches!(error, PluginError::Io(_)));
        assert_eq!(error.to_string(), "Plugin I/O error: gone");
        assert_eq!(error.source().unwrap().to_string(), "gone");
    }

    #[test]
    fn json_errors_convert_and_keep_their_source() {
        let cause = serde_json::from_str::<u32>("nope").unwrap_err();
        let message = cause.to_string();
        let error = PluginError::from(cause);
        assert!(matches!(error, PluginError::Serialization(_)));
        assert_eq!(
            error.to_string(),
            format!("Plugin serialization error: {message}")
        );
        assert!(error.source().is_some());
    }

    #[test]
    fn interior_nul_errors_convert_and_keep_their_source() {
        let cause = std::ffi::CString::new("a\0b").unwrap_err();
        let error = PluginError::from(cause);
        assert!(matches!(error, PluginError::CString(_)));
        assert!(error.to_string().starts_with("CString conversion error"));
        assert!(error.source().is_some());
    }

    #[test]
    #[allow(
        unsafe_code,
        reason = "loads a library that does not exist, to build a real error"
    )]
    fn library_loading_errors_convert_and_keep_their_source() {
        // SAFETY: loading a library that does not exist runs no initialisation code.
        let cause = unsafe { libloading::Library::new("ntd-no-such-library-for-tests") }
            .err()
            .unwrap();
        let error = PluginError::from(cause);
        assert!(matches!(error, PluginError::LibraryLoading(_)));
        assert!(error.to_string().starts_with("Dynamic library load error"));
        assert!(error.source().is_some());
    }

    #[test]
    fn a_trust_error_says_why_the_approval_failed() {
        assert_eq!(
            TrustError::NotPending.to_string(),
            "This plugin is no longer pending approval (the file changed?)"
        );
        let io = std::io::Error::other("disk full");
        assert_eq!(TrustError::from(io).to_string(), "disk full");
    }

    #[test]
    fn a_failed_removal_keeps_the_path_and_the_cause() {
        let error = DeletePluginError::Remove {
            path: PathBuf::from("plugins/filter.dll"),
            source: std::io::Error::other("in use"),
        };
        assert_eq!(
            error.to_string(),
            format!(
                "Failed to delete plugin file at {}: in use",
                PathBuf::from("plugins/filter.dll").display()
            )
        );
        assert!(error.source().is_some());
    }
}
