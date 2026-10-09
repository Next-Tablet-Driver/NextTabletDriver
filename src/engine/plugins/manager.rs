//! # Plugin Manager
//!
//! Scans system and application directories for external plugins, manages the lifecycle
//! of loaded shared libraries, and produces configured [`PluginInstance`]s for the filtering pipeline.

use super::instance::PluginInstance;
use super::loader::{LoadedLibrary, PluginLoader};
use super::trust::{self, TrustStore, UntrustedPlugin};
use crate::core::config::models::MappingConfig;
use crate::engine::state::LockRecoveryExt;
use crate::filters::FilterPipeline;
use ntd_plugin_api::PluginManifest;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[cfg(target_os = "windows")]
const LIB_EXTENSION: &str = "dll";
#[cfg(not(target_os = "windows"))]
const LIB_EXTENSION: &str = "so";

/// Central manager and registry for dynamic `NextTabletDriver` plugins.
pub struct PluginManager {
    loaded: RwLock<HashMap<String, Arc<LoadedLibrary>>>,
    /// Libraries found on disk that are not trusted and were therefore not loaded.
    untrusted: RwLock<Vec<UntrustedPlugin>>,
    /// User-approved library hashes. Only libraries listed here (or officially signed) are loaded.
    trust: RwLock<TrustStore>,
    search_paths: Vec<PathBuf>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// Creates a new `PluginManager` initialized with the standard user plugins path and discovers plugins.
    #[must_use]
    pub fn new() -> Self {
        let search_paths = vec![crate::settings::get_plugins_dir()];

        Self::with_paths_and_store(
            search_paths,
            TrustStore::load(crate::settings::get_settings_dir().join("trusted_plugins.json")),
        )
    }

    /// Creates a new `PluginManager` with custom search paths, using the default trust store.
    #[must_use]
    pub fn with_paths(search_paths: Vec<PathBuf>) -> Self {
        Self::with_paths_and_store(
            search_paths,
            TrustStore::load(crate::settings::get_settings_dir().join("trusted_plugins.json")),
        )
    }

    /// Creates a new `PluginManager` with custom search paths and an explicit trust store.
    #[must_use]
    pub fn with_paths_and_store(search_paths: Vec<PathBuf>, trust: TrustStore) -> Self {
        let manager = Self {
            loaded: RwLock::new(HashMap::new()),
            untrusted: RwLock::new(Vec::new()),
            trust: RwLock::new(trust),
            search_paths,
        };
        manager.load_all();
        manager
    }

    /// Returns the primary plugins directory where user-installed plugins should be placed.
    #[must_use]
    pub fn primary_plugins_dir(&self) -> PathBuf {
        self.search_paths
            .first()
            .cloned()
            .unwrap_or_else(crate::settings::get_plugins_dir)
    }

    /// Scans all configured search paths and loads any discoverable plugins.
    pub fn load_all(&self) {
        let mut discovered = HashMap::new();
        let mut untrusted = Vec::new();

        for dir in &self.search_paths {
            log::info!(target: "Plugins", "Searching for plugins in directory: {}", dir.display());
            if !dir.exists() {
                log::info!(target: "Plugins", "Directory does not exist: {}", dir.display());
                continue;
            }

            let Ok(entries) = fs::read_dir(dir) else {
                log::info!(target: "Plugins", "Failed to read directory: {}", dir.display());
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                if path.extension().and_then(|e| e.to_str()) != Some(LIB_EXTENSION) {
                    continue;
                }

                let path_display = path.display();

                // Never execute a library the user has not approved: loading runs its
                // initialisation code immediately.
                let sha256 = match trust::sha256_file(&path) {
                    Ok(hash) => hash,
                    Err(e) => {
                        log::warn!(target: "Plugins", "Cannot read plugin library {path_display}: {e}");
                        continue;
                    }
                };
                let is_trusted = self
                    .trust
                    .read()
                    .unwrap_or_log("plugin_trust")
                    .is_trusted(&sha256)
                    || trust::has_valid_official_signature(&path);
                if !is_trusted {
                    log::warn!(
                        target: "Plugins",
                        "Skipping untrusted plugin library {path_display} (sha256 {sha256}); approve it in the Filters tab to load it"
                    );
                    untrusted.push(UntrustedPlugin {
                        file_name: entry.file_name().to_string_lossy().into_owned(),
                        sha256,
                    });
                    continue;
                }

                match PluginLoader::load(&path) {
                    Ok(lib) => {
                        let id = lib.manifest.id.clone();
                        match discovered.entry(id) {
                            std::collections::hash_map::Entry::Occupied(entry) => {
                                let key = entry.key();
                                log::warn!(
                                    target: "Plugins",
                                    "Duplicate plugin ID '{key}' discovered at {path_display}; skipping in favor of earlier match"
                                );
                            }
                            std::collections::hash_map::Entry::Vacant(entry) => {
                                entry.insert(lib);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            target: "Plugins",
                            "Failed to load potential plugin library at {path_display}: {e}"
                        );
                    }
                }
            }
        }

        let mut lock = self.loaded.write().unwrap_or_log("loaded_plugins");
        *lock = discovered;
        drop(lock);
        *self.untrusted.write().unwrap_or_log("untrusted_plugins") = untrusted;
    }

    /// Libraries present in the plugins folder that were skipped because they are not trusted.
    #[must_use]
    pub fn untrusted_plugins(&self) -> Vec<UntrustedPlugin> {
        self.untrusted
            .read()
            .unwrap_or_log("untrusted_plugins")
            .clone()
    }

    /// Approves a library by content hash and reloads, so it is loaded if the file on disk
    /// still matches the hash the user reviewed.
    ///
    /// # Errors
    /// Returns an error string if no pending library has this hash (for instance because the
    /// file changed since it was listed) or if the trust store cannot be written.
    pub fn trust_plugin(&self, sha256: &str) -> Result<(), String> {
        let file_name = self
            .untrusted_plugins()
            .into_iter()
            .find(|p| p.sha256 == sha256)
            .map(|p| p.file_name)
            .ok_or_else(|| {
                "This plugin is no longer pending approval (the file changed?)".to_string()
            })?;
        self.trust
            .write()
            .unwrap_or_log("plugin_trust")
            .trust(sha256, &file_name)?;
        self.reload();
        Ok(())
    }

    /// Approves a library file that was just installed by the user through the app.
    ///
    /// # Errors
    /// Returns an error string if the file cannot be hashed or the trust store cannot be written.
    pub fn trust_installed_file(&self, path: &std::path::Path) -> Result<(), String> {
        let sha256 = trust::sha256_file(path).map_err(|e| e.to_string())?;
        let file_name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        self.trust
            .write()
            .unwrap_or_log("plugin_trust")
            .trust(&sha256, &file_name)
    }

    /// Reloads all plugins from disk, refreshing manifests and dynamic code.
    pub fn reload(&self) {
        log::info!(target: "Plugins", "Reloading all dynamic plugins...");
        self.load_all();
    }

    /// Returns a list of all currently loaded plugin manifests, sorted by name.
    #[must_use]
    pub fn loaded_manifests(&self) -> Vec<PluginManifest> {
        let mut manifests: Vec<PluginManifest> = {
            let lock = self.loaded.read().unwrap_or_log("loaded_plugins");
            lock.values().map(|l| l.manifest.clone()).collect()
        };
        manifests.sort_by(|a, b| a.name.cmp(&b.name));
        manifests
    }

    /// Returns the manifest for a specific plugin ID, if loaded.
    #[must_use]
    pub fn get_manifest(&self, id: &str) -> Option<PluginManifest> {
        let lock = self.loaded.read().unwrap_or_log("loaded_plugins");
        lock.get(id).map(|l| l.manifest.clone())
    }

    /// Deletes a plugin's dynamic library file from disk and unloads it.
    ///
    /// # Errors
    /// Returns an error string if the plugin is not loaded or if file deletion fails.
    pub fn delete_plugin(&self, id: &str) -> Result<PathBuf, String> {
        let (lib_path, _lib) = {
            let mut lock = self.loaded.write().unwrap_or_log("loaded_plugins");
            let lib = lock
                .remove(id)
                .ok_or_else(|| format!("Plugin '{id}' is not loaded"))?;
            drop(lock);
            (lib.path.clone(), lib)
        };

        if let Err(e) = std::fs::remove_file(&lib_path) {
            #[cfg(target_os = "windows")]
            {
                // On Windows, if the file is locked by an OS handle, rename it first
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis());
                let temp_path = lib_path.with_extension(format!("deleted_{timestamp}.tmp"));
                if std::fs::rename(&lib_path, &temp_path).is_ok() {
                    let _ = std::fs::remove_file(&temp_path);
                    self.reload();
                    return Ok(lib_path);
                }
            }
            return Err(format!(
                "Failed to delete plugin file at {}: {e}",
                lib_path.display()
            ));
        }

        self.reload();
        Ok(lib_path)
    }

    /// Instantiates an active [`PluginInstance`] for the given plugin ID.
    #[must_use]
    pub fn create_instance(&self, id: &str) -> Option<PluginInstance> {
        let lib = {
            let lock = self.loaded.read().unwrap_or_log("loaded_plugins");
            lock.get(id).cloned()?
        };
        match PluginInstance::new(lib) {
            Ok(inst) => Some(inst),
            Err(e) => {
                log::error!(target: "Plugins", "Failed to create instance of plugin '{id}': {e}");
                None
            }
        }
    }

    /// Creates a complete [`FilterPipeline`] populated with instances of all loaded plugins,
    /// configured with preferences from `config.plugins`.
    #[must_use]
    pub fn create_pipeline(&self, config: &MappingConfig) -> FilterPipeline {
        FilterPipeline::from_manager(self, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntd_plugin_api::{PluginContext, PluginPacket};

    #[test]
    fn test_untrusted_library_is_never_loaded() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!("ntd_plugins_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let lib_name = format!("evil.{LIB_EXTENSION}");
        // Not a real library: if the manager tried to load it, it would log a load error;
        // what matters is that it is reported as pending approval and never reaches the loader.
        std::fs::write(dir.join(&lib_name), b"not a library").unwrap();

        let store_path = dir.join("trusted_plugins.json");
        let manager = PluginManager::with_paths_and_store(
            vec![dir.clone()],
            TrustStore::load(store_path.clone()),
        );
        let pending = manager.untrusted_plugins();
        assert_eq!(pending.len(), 1);
        let first = pending.first().unwrap();
        assert_eq!(first.file_name, lib_name);
        assert!(manager.loaded_manifests().is_empty());

        // Approving a hash that is not pending is refused.
        assert!(manager.trust_plugin("0000").is_err());

        // Approving the real hash moves it to the loader (which then rejects the fake binary).
        manager.trust_plugin(&first.sha256).unwrap();
        assert!(manager.untrusted_plugins().is_empty());
        assert!(TrustStore::load(store_path).is_trusted(&first.sha256));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_manager_pipeline_creation() {
        let dev_dir = PathBuf::from("plugins/radial_follow/target/debug");
        let manager = if dev_dir.is_dir() {
            PluginManager::with_paths(vec![dev_dir])
        } else {
            PluginManager::new()
        };
        let mut config = MappingConfig::default();

        let pipeline = manager.create_pipeline(&config);
        let has_radial = pipeline.entries.iter().any(|e| e.id == "radial_follow");
        if has_radial {
            let entry = pipeline
                .entries
                .iter()
                .find(|e| e.id == "radial_follow")
                .unwrap();
            assert!(!entry.enabled);

            let mut settings = crate::core::config::models::DynamicPluginSettings {
                enabled: true,
                properties: HashMap::new(),
            };
            settings
                .properties
                .insert("outer_radius".to_string(), serde_json::json!(8.0));
            config.plugins.insert("radial_follow".to_string(), settings);

            let mut active_pipeline = manager.create_pipeline(&config);
            let active_entry = active_pipeline
                .entries
                .iter()
                .find(|e| e.id == "radial_follow")
                .unwrap();
            assert!(active_entry.enabled);

            let mut packet = PluginPacket {
                u: 0.5,
                v: 0.5,
                pressure: 0.5,
                tilt_x: 0,
                tilt_y: 0,
                is_down: true,
                timestamp_ns: 0,
            };
            let context = PluginContext::default();
            active_pipeline.process(&mut packet, &context);
            assert!((packet.u - 0.5).abs() < f32::EPSILON);
        }
    }
}
