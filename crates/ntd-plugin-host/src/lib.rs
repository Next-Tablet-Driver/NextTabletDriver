//! # Plugin host
//!
//! Provides dynamic plugin discovery, loading, verification, and lifecycle management.
//! Communicates with external plugins via the C-ABI (v3) defined in `ntd_plugin_api`.
//!
//! The order is fixed: a library is hashed and its signature checked **before** the OS loader
//! opens it (loading runs its initialisation code); only then are the ABI version and the target
//! checked. A library built for another ABI (v2 plugins) or another target is refused with a
//! message that says so. A plugin that fails while running is disabled and the pipeline goes on
//! without it.

pub mod error;
pub mod filter;
pub mod instance;
pub mod loader;
pub mod manager;
pub mod trust;

pub use error::PluginError;
pub use filter::PluginFilter;
pub use instance::PluginInstance;
pub use loader::{LoadedLibrary, PluginLoader};
pub use manager::PluginManager;
pub use trust::{TrustStore, UntrustedPlugin};
