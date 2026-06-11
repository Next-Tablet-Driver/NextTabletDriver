pub mod models;
pub mod snapshot;

pub use models::*;
pub use snapshot::*;

use crate::t;

use crate::core::config::models::MappingConfig;
use crossbeam_channel::Receiver;
use display_info::DisplayInfo;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use crate::app::autoupdate::UpdateStatus;
use crate::drivers::TabletData;
use crate::engine::state::{LockRecoveryExt, SharedState};

pub type ThemeStoreResult = Result<Vec<String>, String>;

/// The core application state structure used by the `eframe` (egui) integration.
#[allow(clippy::struct_excessive_bools)]
pub struct TabletMapperApp {
    /// Reference-counted handle to the thread-safe engine/driver state.
    pub shared: Arc<SharedState>,

    /// List of detected physical monitors/displays on the host system.
    pub displays: Vec<DisplayInfo>,
    /// Instant of the last update/repaint cycle.
    pub last_update: Instant,
    /// Instant when the active configuration was last logged.
    pub last_config_log: Instant,
    /// Tracks profile state identity (name, filepath, and saved/unsaved status).
    pub profile: ProfileState,
    /// The currently selected app tab/view in the UI panel.
    pub active_tab: AppTab,

    /// Channel receiver for tablet input packets streamed from the driver thread.
    pub tablet_receiver: Receiver<TabletData>,
    /// Channel receiver for software auto-update statuses.
    pub update_receiver: Receiver<UpdateStatus>,
    /// Channel sender for publishing auto-update status events.
    pub update_sender: crossbeam_channel::Sender<UpdateStatus>,
    /// Current cached auto-update status.
    pub update_status: UpdateStatus,

    /// Channel sender to trigger asynchronous config writes to the settings file.
    pub save_sender: crossbeam_channel::Sender<MappingConfig>,

    /// Queue of active toast notifications rendered in the overlay.
    pub toasts: Vec<Toast>,

    /// Display name of the filter currently selected in the Filters tab.
    pub selected_filter: String,

    /// Toggle to render the floating developer debug details panel.
    pub show_debugger: bool,
    /// Toggle to display real-time latency statistics.
    pub show_latency_stats: bool,
    /// Real-time frame paint speed and packet latency diagnostics.
    pub metrics: Metrics,

    /// Remembers if the window was minimized in the last update frame.
    pub was_minimized: bool,

    /// Sub-string filter for searching the console logs.
    pub console_search: String,
    /// Show INFO level logs in the console panel.
    pub console_show_info: bool,
    /// Show WARN level logs in the console panel.
    pub console_show_warn: bool,
    /// Show ERROR level logs in the console panel.
    pub console_show_error: bool,
    /// Show DEBUG level logs in the console panel.
    pub console_show_debug: bool,
    /// Automatically scroll to the bottom when a new log arrives.
    pub console_autoscroll: bool,

    /// Monotonically increasing sequence number used to track if new logs have been received
    /// and if the console cache needs to be re-filtered and regenerated.
    pub console_cache_log_sequence: u64,
    /// The search term used to generate the current console cache.
    pub console_cache_search: String,
    /// The filter switches used to generate the current console cache: `(info, warn, error, debug)`.
    pub console_cache_filters: (bool, bool, bool, bool),
    /// List of pre-filtered log entries currently loaded in the console UI.
    pub console_cache_filtered: Vec<crate::logger::LogEntry>,

    // Theme Store State
    pub theme_store_open: bool,
    pub theme_store_loading: bool,
    pub theme_store_list: std::sync::Arc<std::sync::Mutex<Option<ThemeStoreResult>>>,

    /// Toggle to render the close confirmation dialog modal.
    pub show_close_confirm: bool,
    /// If true, bypasses close confirmation dialog and exits immediately.
    pub force_close: bool,

    /// Set to true on Linux if the required udev rules are not installed.
    pub missing_udev_rules: bool,
}

const MAX_TOASTS: usize = 3;

impl TabletMapperApp {
    /// Pushes a new toast notification message into the display queue.
    ///
    /// Deduplicates duplicate messages and respects `MAX_TOASTS`.
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
            .set_directory(crate::settings::get_settings_dir())
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
            let mut shared_config = self.shared.config.write().unwrap_or_log("config");
            let theme = shared_config.theme.clone();
            let run_at_startup = shared_config.run_at_startup;
            let language = shared_config.language;
            *shared_config = MappingConfig::default();
            shared_config.theme = theme;
            shared_config.run_at_startup = run_at_startup;
            shared_config.language = language;
            self.shared
                .config_version
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

    pub fn get_filtered_logs(&mut self) -> (usize, &[crate::logger::LogEntry]) {
        let logs = crate::logger::LOG_BUFFER.read().unwrap_or_log("logs");
        let current_filters = (
            self.console_show_info,
            self.console_show_warn,
            self.console_show_error,
            self.console_show_debug,
        );
        let current_sequence =
            crate::logger::LOG_SEQUENCE.load(std::sync::atomic::Ordering::Acquire);
        if self.console_cache_log_sequence == current_sequence
            && self.console_cache_search == self.console_search
            && self.console_cache_filters == current_filters
        {
            return (logs.len(), &self.console_cache_filtered);
        }
        let search_lower = self.console_search.to_lowercase();
        let mut filtered: Vec<_> = logs
            .iter()
            .filter(|log| {
                let level_match = match log.level.as_str() {
                    "Info" => self.console_show_info,
                    "Warn" => self.console_show_warn,
                    "Error" => self.console_show_error,
                    "Debug" => self.console_show_debug,
                    _ => true,
                };
                if !level_match {
                    return false;
                }
                if search_lower.is_empty() {
                    return true;
                }
                log.search_text.contains(&search_lower)
            })
            .cloned()
            .collect();
        filtered.reverse();
        let all_count = logs.len();
        drop(logs);
        self.console_cache_filtered = filtered;
        self.console_cache_log_sequence = current_sequence;
        self.console_cache_search = self.console_search.clone();
        self.console_cache_filters = current_filters;

        (all_count, &self.console_cache_filtered)
    }

    pub fn start_update(&mut self) {
        if let UpdateStatus::Available(release) = &self.update_status {
            let release_clone = release.clone();
            let sender = self.update_sender.clone();
            std::thread::spawn(move || {
                if let Err(e) =
                    crate::app::autoupdate::download_and_install(&release_clone, &sender)
                {
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
            Ok(Some(release)) => {
                let _ = sender.send(UpdateStatus::Available(release));
            }
            Ok(None) => {
                let _ = sender.send(UpdateStatus::Idle);
            }
            Err(e) => {
                let _ = sender.send(UpdateStatus::Error(e.to_string()));
            }
        });
    }

    fn apply_config(&self, cfg: MappingConfig) {
        {
            let mut shared_config = self.shared.config.write().unwrap_or_log("config");
            *shared_config = cfg.clone();
            self.shared
                .config_version
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            drop(shared_config);
        }
        let _ = self.save_sender.try_send(cfg);
    }
}
