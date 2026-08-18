//! Profile persistence actions: loading, saving, resetting, exporting, and
//! importing (native and `OpenTabletDriver`) the active tablet configuration.

use super::TabletMapperApp;
use super::models::ToastLevel;
use crate::core::config::models::MappingConfig;
use crate::engine::state::LockRecoveryExt;
use crate::t;
use std::path::Path;

impl TabletMapperApp {
    /// Loads a profile configuration from the specified JSON file path.
    ///
    /// Automatically repairs invalid parameters, updates active session metadata,
    /// and posts corresponding toast notifications indicating success/failure.
    pub fn load_profile_at_path(&mut self, path: &Path) {
        match crate::settings::load_settings_from_file(path) {
            Ok((cfg, corrections)) => {
                self.apply_config(cfg.clone());
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    self.profile.name = name.to_string();
                }
                self.profile.path = Some(path.to_path_buf());
                self.profile.mark_saved(&cfg);
                crate::settings::save_session_meta(&crate::settings::SessionMeta {
                    profile_name: self.profile.name.clone(),
                    profile_path: self.profile.path.clone(),
                });
                if !corrections.is_empty() {
                    self.push_toast(
                        t!("toast.config_repaired", count = corrections.len()),
                        ToastLevel::Warning,
                    );
                }
                self.push_toast(
                    t!("toast.profile_loaded", name = &self.profile.name),
                    ToastLevel::Info,
                );
            }
            Err(e) => {
                self.push_toast(t!("toast.load_failed", error = e), ToastLevel::Error);
            }
        }
    }

    /// Triggers an OS native file picker modal to select and load a profile JSON file.
    pub fn load_settings(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(crate::settings::get_profiles_dir())
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            self.load_profile_at_path(&path);
        }
    }

    /// Saves the current configuration to the active profile file path.
    pub fn save_settings(&mut self, config: &MappingConfig) {
        let config = config.clone();
        if let Some(ref path) = self.profile.path {
            match crate::settings::save_to_path(path, &config) {
                Ok(()) => {
                    self.profile.mark_saved(&config);
                    let _ = self.save_sender.try_send(config);
                    self.push_toast(t!("toast.settings_saved"), ToastLevel::Info);
                }
                Err(e) => {
                    self.push_toast(t!("toast.save_failed", error = e), ToastLevel::Error);
                }
            }
        } else {
            self.save_settings_as(config);
        }
    }

    pub fn save_settings_as(&mut self, config: MappingConfig) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(crate::settings::get_profiles_dir())
            .add_filter("JSON", &["json"])
            .save_file()
        {
            match crate::settings::save_to_path(&path, &config) {
                Ok(()) => {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        self.profile.name = name.to_string();
                    }
                    self.profile.path = Some(path);
                    self.profile.mark_saved(&config);
                    let _ = self.save_sender.try_send(config);
                    crate::settings::save_session_meta(&crate::settings::SessionMeta {
                        profile_name: self.profile.name.clone(),
                        profile_path: self.profile.path.clone(),
                    });
                    crate::app::telemetry::capture_event(
                        "profile_saved_as",
                        Some(serde_json::json!({ "profile_name": self.profile.name })),
                    );
                    self.push_toast(t!("toast.settings_saved"), ToastLevel::Info);
                }
                Err(e) => {
                    self.push_toast(t!("toast.save_failed", error = e), ToastLevel::Error);
                }
            }
        }
    }

    pub fn reset_to_default(&mut self) {
        {
            let mut shared_config = self.shared.config.mapping.write().unwrap_or_log("config");
            let run_at_startup = shared_config.run_at_startup;
            *shared_config = MappingConfig::default();
            shared_config.run_at_startup = run_at_startup;
            self.shared
                .config
                .version
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            drop(shared_config);
        }
        self.push_toast(t!("toast.reset_default"), ToastLevel::Info);
    }

    pub fn export_settings(&mut self, config: &MappingConfig) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("settings_export.json")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            match crate::settings::save_to_path(&path, config) {
                Ok(()) => self.push_toast(t!("toast.settings_exported"), ToastLevel::Info),
                Err(e) => self.push_toast(t!("toast.export_failed", error = e), ToastLevel::Error),
            }
        }
    }

    pub fn import_settings(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match crate::settings::load_settings_from_file(&path) {
                Ok((cfg, corrections)) => {
                    self.apply_config(cfg);
                    if !corrections.is_empty() {
                        self.push_toast(
                            t!("toast.import_repaired", count = corrections.len()),
                            ToastLevel::Warning,
                        );
                    }
                }
                Err(e) => self.push_toast(t!("toast.import_failed", error = e), ToastLevel::Error),
            }
        }
    }

    pub fn import_otd_settings(&mut self) {
        let mut dialog = rfd::FileDialog::new().add_filter("OTD JSON", &["json"]);

        if let Some(base_dirs) = directories::BaseDirs::new() {
            let local_app_data = base_dirs.data_local_dir();
            let otd_dir = local_app_data.join("OpenTabletDriver");
            if otd_dir.exists() {
                dialog = dialog.set_directory(&otd_dir);
            }
        }

        if let Some(path) = dialog.pick_file() {
            match crate::settings::otd_import::import_otd_profile(&path) {
                Ok(cfg) => {
                    self.apply_config(cfg);
                    self.push_toast(
                        "OTD settings imported successfully".to_string(),
                        ToastLevel::Info,
                    );
                    crate::app::telemetry::capture_event("otd_imported", None);
                }
                Err(e) => self.push_toast(
                    format!("Failed to import OTD settings: {e}"),
                    ToastLevel::Error,
                ),
            }
        }
    }

    fn apply_config(&self, cfg: MappingConfig) {
        {
            let mut shared_config = self.shared.config.mapping.write().unwrap_or_log("config");
            *shared_config = cfg.clone();
            self.shared
                .config
                .version
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            drop(shared_config);
        }
        let _ = self.save_sender.try_send(cfg);
    }
}
