//! # Plugin System Errors
//!
//! Strongly-typed errors encountered during dynamic library loading,
//! ABI handshake negotiation, property synchronization, and execution.

use std::fmt;

/// Errors produced during plugin loading, validation, or invocation.
#[derive(Debug)]
pub enum PluginError {
    /// An I/O error occurred while inspecting plugin files or directories.
    Io(std::io::Error),
    /// A dynamic library loading error occurred via `libloading`.
    LibraryLoading(libloading::Error),
    /// The plugin binary did not present the expected magic identifier.
    InvalidMagic {
        /// The expected magic constant.
        expected: u32,
        /// The magic constant retrieved from the plugin handshake.
        found: u32,
    },
    /// The plugin binary targets an incompatible ABI version.
    IncompatibleAbiVersion {
        /// The expected host ABI version.
        expected: u32,
        /// The ABI version requested by the plugin.
        found: u32,
    },
    /// A required FFI symbol was not exported by the plugin library.
    MissingSymbol(&'static str),
    /// The plugin manifest could not be read or deserialized.
    ManifestError(String),
    /// Instantiation of the plugin instance via `ntd_plugin_create` failed.
    InstanceCreationFailed,
    /// An error occurred while setting a plugin configuration property.
    PropertyError(String),
    /// JSON serialization or deserialization failed.
    Serialization(serde_json::Error),
    /// String conversion to null-terminated C string failed due to interior nulls.
    CString(std::ffi::NulError),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "Plugin I/O error: {err}"),
            Self::LibraryLoading(err) => write!(f, "Dynamic library load error: {err}"),
            Self::InvalidMagic { expected, found } => {
                write!(
                    f,
                    "Invalid plugin magic identifier: expected 0x{expected:08X}, found 0x{found:08X}"
                )
            }
            Self::IncompatibleAbiVersion { expected, found } => {
                write!(
                    f,
                    "Incompatible plugin ABI version: expected {expected}, found {found}"
                )
            }
            Self::MissingSymbol(name) => write!(f, "Missing required plugin FFI symbol: '{name}'"),
            Self::ManifestError(msg) => write!(f, "Failed to parse plugin manifest: {msg}"),
            Self::InstanceCreationFailed => {
                write!(f, "Plugin instance creation returned a null pointer")
            }
            Self::PropertyError(key) => write!(f, "Failed to set plugin property '{key}'"),
            Self::Serialization(err) => write!(f, "Plugin serialization error: {err}"),
            Self::CString(err) => write!(f, "CString conversion error: {err}"),
        }
    }
}

impl std::error::Error for PluginError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::LibraryLoading(err) => Some(err),
            Self::Serialization(err) => Some(err),
            Self::CString(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PluginError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<libloading::Error> for PluginError {
    fn from(err: libloading::Error) -> Self {
        Self::LibraryLoading(err)
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

impl From<std::ffi::NulError> for PluginError {
    fn from(err: std::ffi::NulError) -> Self {
        Self::CString(err)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
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
}
