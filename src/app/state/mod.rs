pub mod models;
pub mod snapshot;

pub use models::*;
pub use snapshot::*;

use crate::core::config::models::MappingConfig;
use display_info::DisplayInfo;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use crossbeam_channel::Receiver;

use crate::app::autoupdate::UpdateStatus;
use crate::drivers::TabletData;
use crate::engine::state::{LockRecoveryExt, SharedState};

/// The core application state structure used by the `eframe` (egui) integration.
pub struct TabletMapperApp {
    // Shared State
    pub shared: Arc<SharedState>,

    // UI Local State
    pub displays: Vec<DisplayInfo>,
    pub last_update: Instant,
    pub last_config_log: Instant,
    pub profile: ProfileState,
    pub active_tab: AppTab,

    // Event Receivers
    pub tablet_receiver: Receiver<TabletData>,
    pub update_receiver: Receiver<UpdateStatus>,
    pub update_sender: crossbeam_channel::Sender<UpdateStatus>,
    pub update_status: UpdateStatus,

    // Background Saver
    pub save_sender: crossbeam_channel::Sender<MappingConfig>,

    // Toast Notifications
    pub toasts: Vec<Toast>,

    // Filters UI State
    pub selected_filter: String,

    // Debugger & Performance UI State
    pub show_debugger: bool,
    pub show_latency_stats: bool,
    pub metrics: Metrics,

    pub was_minimized: bool,

    // Console State
    pub console_search: String,
    pub console_show_info: bool,
    pub console_show_warn: bool,
    pub console_show_error: bool,
    pub console_show_debug: bool,
    pub console_autoscroll: bool,
    
    // Console Cache
    pub console_cache_log_count: usize,
    pub console_cache_search: String,
    pub console_cache_filters: (bool, bool, bool, bool),
    pub console_cache_filtered: Vec<crate::logger::LogEntry>,
    pub console_cache_full_text: String,

    // System Tray
    pub tray_icon: Option<tray_icon::TrayIcon>,

    // Close Confirmation
    pub show_close_confirm: bool,
    pub force_close: bool,
}

const MAX_TOASTS: usize = 3;

impl TabletMapperApp {
    pub fn push_toast(&mut self, message: String, level: ToastLevel) {
        if self.toasts.iter().any(|t| t.message == message) {
            return;
        }
        if self.toasts.len() >= MAX_TOASTS {
            self.toasts.remove(0);
        }
        self.toasts.push(Toast {
            message,
            level,
            created_at: Instant::now(),
        });
    }

    pub fn load_profile_at_path(&mut self, path: PathBuf) {
        match crate::settings::load_settings_from_file(&path) {
            Ok((cfg, corrections)) => {
                self.apply_config(cfg.clone());
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    self.profile.name = name.to_string();
                }
                self.profile.path = Some(path.clone());
                self.profile.mark_saved(&cfg);
                crate::settings::save_session_meta(&crate::settings::SessionMeta {
                    profile_name: self.profile.name.clone(),
                    profile_path: self.profile.path.clone(),
                });
                if !corrections.is_empty() {
                    self.push_toast(format!("Config repaired: {} field(s) reset to defaults", corrections.len()), ToastLevel::Warning);
                }
                self.push_toast(format!("Loaded profile: {}", self.profile.name), ToastLevel::Info);
            }
            Err(e) => {
                self.push_toast(format!("Failed to load profile: {}", e), ToastLevel::Error);
            }
        }
    }

    pub fn load_settings(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(crate::settings::get_settings_dir())
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            self.load_profile_at_path(path);
        }
    }

    pub fn save_settings(&mut self, config: &MappingConfig) {
        let config = config.clone();
        if let Some(ref path) = self.profile.path {
            match crate::settings::save_to_path(path, &config) {
                Ok(()) => {
                    self.profile.mark_saved(&config);
                    let _ = self.save_sender.try_send(config);
                    self.push_toast("Settings saved".to_string(), ToastLevel::Info);
                }
                Err(e) => {
                    self.push_toast(format!("Failed to save: {}", e), ToastLevel::Error);
                }
            }
        } else {
            self.save_settings_as(config);
        }
    }

    pub fn save_settings_as(&mut self, config: MappingConfig) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(crate::settings::get_settings_dir())
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
                    self.push_toast("Settings saved".to_string(), ToastLevel::Info);
                }
                Err(e) => {
                    self.push_toast(format!("Failed to save: {}", e), ToastLevel::Error);
                }
            }
        }
    }

    pub fn reset_to_default(&mut self) {
        {
            let mut shared_config = self.shared.config.write().unwrap_or_log("config");
            let theme = shared_config.theme;
            let run_at_startup = shared_config.run_at_startup;
            *shared_config = MappingConfig::default();
            shared_config.theme = theme;
            shared_config.run_at_startup = run_at_startup;
            self.shared.config_version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        self.push_toast("Settings reset to default (Unsaved)".to_string(), ToastLevel::Info);
    }

    pub fn export_settings(&mut self, config: &MappingConfig) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("settings_export.json")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            match crate::settings::save_to_path(&path, config) {
                Ok(()) => self.push_toast("Settings exported".to_string(), ToastLevel::Info),
                Err(e) => self.push_toast(format!("Export failed: {}", e), ToastLevel::Error),
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
                        self.push_toast(format!("Imported config repaired: {} field(s) reset", corrections.len()), ToastLevel::Warning);
                    }
                }
                Err(e) => self.push_toast(format!("Import failed: {}", e), ToastLevel::Error),
            }
        }
    }

    pub fn get_filtered_logs(&mut self) -> (usize, &[crate::logger::LogEntry], &str) {
        let logs = crate::logger::LOG_BUFFER.read().unwrap_or_log("logs");
        let current_filters = (self.console_show_info, self.console_show_warn, self.console_show_error, self.console_show_debug);
        if self.console_cache_log_count == logs.len() && self.console_cache_search == self.console_search && self.console_cache_filters == current_filters {
            return (self.console_cache_log_count, &self.console_cache_filtered, &self.console_cache_full_text);
        }
        let search_lower = self.console_search.to_lowercase();
        let mut filtered: Vec<_> = logs.iter().filter(|log| {
            let level_match = match log.level.as_str() { "Info" => self.console_show_info, "Warn" => self.console_show_warn, "Error" => self.console_show_error, "Debug" => self.console_show_debug, _ => true };
            if !level_match { return false; }
            if search_lower.is_empty() { return true; }
            log.message.to_lowercase().contains(&search_lower) || log.group.to_lowercase().contains(&search_lower)
        }).cloned().collect();
        filtered.reverse();
        let full_text = logs.iter().map(|l| format!("[{}] {} [{}] {}", l.time, l.level, l.group, l.message)).collect::<Vec<_>>().join("\n");
        self.console_cache_log_count = logs.len();
        self.console_cache_search = self.console_search.clone();
        self.console_cache_filters = current_filters;
        self.console_cache_filtered = filtered;
        self.console_cache_full_text = full_text;
        (self.console_cache_log_count, &self.console_cache_filtered, &self.console_cache_full_text)
    }

    pub fn start_update(&mut self) {
        if let UpdateStatus::Available(release) = &self.update_status {
            let release_clone = release.clone();
            let sender = self.update_sender.clone();
            std::thread::spawn(move || {
                if let Err(e) = crate::app::autoupdate::download_and_install(release_clone, sender.clone()) {
                    let _ = sender.send(UpdateStatus::Error(e.to_string()));
                }
            });
            self.update_status = UpdateStatus::Downloading(0.0);
        }
    }

    pub fn dismiss_update(&mut self) {
        self.update_status = UpdateStatus::Idle;
    }

    pub fn check_for_updates(&mut self) {
        let sender = self.update_sender.clone();
        self.update_status = UpdateStatus::Checking;
        std::thread::spawn(move || match crate::app::autoupdate::check_for_updates() {
            Ok(Some(release)) => { let _ = sender.send(UpdateStatus::Available(release)); }
            Ok(None) => { let _ = sender.send(UpdateStatus::Idle); }
            Err(e) => { let _ = sender.send(UpdateStatus::Error(e.to_string())); }
        });
    }

    fn apply_config(&self, cfg: MappingConfig) { 
        {
            let mut shared_config = self.shared.config.write().unwrap_or_log("config");
            *shared_config = cfg.clone();
            self.shared.config_version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        let _ = self.save_sender.try_send(cfg);
    }
}
