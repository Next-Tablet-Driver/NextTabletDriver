//! # Dynamic Plugins System
//!
//! Provides dynamic plugin discovery, loading, verification, and lifecycle management.
//! Communicates with external plugins via the C-ABI defined in `ntd_plugin_api`.

pub mod error;
pub mod instance;
pub mod loader;
pub mod manager;
pub mod trust;

pub use error::PluginError;
pub use instance::PluginInstance;
pub use loader::{LoadedLibrary, PluginLoader};
pub use manager::PluginManager;
pub use trust::{TrustStore, UntrustedPlugin};
