//! # Application State
//!
//! This module defines the primary state structures for the GUI application.
//! It contains the main `TabletMapperApp` struct which holds the UI state,
//! shared engine state, and communication channels.

use crate::core::config::models::MappingConfig;
use display_info::DisplayInfo;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::app::autoupdate::UpdateStatus;
use crate::drivers::TabletData;
use crate::engine::state::{LockRecoveryExt, SharedState};
use crossbeam_channel::Receiver;

/// An immutable, lock-free snapshot of the application state for a single UI frame.
///
/// This pattern solves three main issues:
/// 1. **Contention**: Reduces the number of `read()` calls on `RwLock`s during a frame.
/// 2. **Consistency**: Ensures all UI panels see exactly the same state for a given frame.
/// 3. **Safety**: Handles poisoned locks once per frame at a central point.
#[derive(Clone, Debug)]
pub struct UiSnapshot {
    pub tablet_name: String,
    pub tablet_vid: u16,
    pub tablet_pid: u16,
    pub tablet_data: TabletData,
    pub config: MappingConfig,
    pub physical_size: (f32, f32),
    pub hardware_size: (f32, f32),
    pub stats: crate::drivers::DriverStats,
    pub packet_count: u32,
    pub is_first_run: bool,
}

impl UiSnapshot {
    /// Captures a complete state snapshot from the shared engine state.
    /// This should be called exactly once at the beginning of every `update()` frame.
    pub fn capture(shared: &SharedState) -> Self {
        use std::sync::atomic::Ordering;

        let device = shared.device_state.read().unwrap_or_log("device_state").clone();

        Self {
            tablet_name: device.name,
            tablet_vid: device.vid,
            tablet_pid: device.pid,
            tablet_data: shared.tablet_data.read().unwrap_or_log("tablet_data").clone(),
            config: shared.config.read().unwrap_or_log("config").clone(),
            physical_size: device.physical_size,
            hardware_size: device.hardware_size,
            stats: *shared.stats.read().unwrap_or_log("stats"),
            packet_count: shared.packet_count.load(Ordering::Relaxed),
            is_first_run: *shared.is_first_run.read().unwrap_or_log("is_first_run"),
        }
    }
}

/// Represents the currently active tab in the main application window.
///
/// This enum dictates which UI panel is rendered to the user.
#[derive(PartialEq, Clone, Copy)]
pub enum AppTab {
    /// The main output/mapping tab.
    Output,
    /// Settings for anti-chatter and smoothing filters.
    Filters,
    /// Pen button bindings and pressure curve settings.
    PenSettings,
    /// Real-time tablet events and debugging output.
    Console,
    /// General application settings (startup, updater, etc).
    Settings,
    /// Changelog and update installation dialog.
    Release,
}

/// Tracks the state of the currently active user profile.
pub struct ProfileState {
    /// Display name shown in the UI footer.
    pub name: String,
    /// Absolute path to the profile file on disk. `None` for unsaved sessions.
    pub path: Option<PathBuf>,
    /// Snapshot of the config at the time of the last save/load.
    /// Used to detect dirty (unsaved changes) state.
    pub last_saved: MappingConfig,
}

impl ProfileState {
    /// Returns `true` if the current config differs from the last saved snapshot.
    pub fn is_dirty(&self, current: &MappingConfig) -> bool {
        *current != self.last_saved
    }

    /// Returns the display name for the footer, prefixed with `*` if dirty.
    pub fn display_name(&self, current: &MappingConfig) -> String {
        let base = if self.path.is_some() {
            &self.name
        } else {
            "Unsaved Session"
        };

        if self.is_dirty(current) {
            format!("*{}", base)
        } else {
            base.to_string()
        }
    }

    /// Updates the saved snapshot after a successful save/load.
    pub fn mark_saved(&mut self, config: &MappingConfig) {
        self.last_saved = config.clone();
    }
}

/// Severity level for UI toast notifications.
#[derive(Clone, Copy, PartialEq)]
pub enum ToastLevel {
    Info,
    Warning,
    Error,
}

/// A transient notification displayed in the UI overlay.
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
}

/// Encapsulates performance metrics and latency tracking.
pub struct Metrics {
    pub displayed_hz: f32,
    pub last_hz_update: Instant,
    pub last_packet_count: u32,
    pub ui_latency_ms: f32,
    pub min_ui_latency_ms: f32,
    pub max_ui_latency_ms: f32,
    pub avg_ui_latency_ms: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            displayed_hz: 0.0,
            last_hz_update: Instant::now(),
            last_packet_count: 0,
            ui_latency_ms: 0.0,
            min_ui_latency_ms: f32::MAX,
            max_ui_latency_ms: 0.0,
            avg_ui_latency_ms: 0.0,
        }
    }
}

impl Metrics {
    pub fn update_hz(&mut self, current_packets: u32) {
        let elapsed = self.last_hz_update.elapsed();
        if elapsed >= std::time::Duration::from_millis(200) {
            let delta = current_packets.saturating_sub(self.last_packet_count);
            let hz = delta as f32 / elapsed.as_secs_f32();
            self.displayed_hz += (hz - self.displayed_hz) * 0.3;
            self.last_packet_count = current_packets;
            self.last_hz_update = Instant::now();
        }
    }

    pub fn update_latency(&mut self, latency: f32) {
        self.ui_latency_ms = latency;
        self.min_ui_latency_ms = self.min_ui_latency_ms.min(latency);
        self.max_ui_latency_ms = self.max_ui_latency_ms.max(latency);
        self.avg_ui_latency_ms += (latency - self.avg_ui_latency_ms) * 0.1;
    }

    pub fn reset_ui_latency(&mut self) {
        self.min_ui_latency_ms = f32::MAX;
        self.max_ui_latency_ms = 0.0;
        self.avg_ui_latency_ms = 0.0;
    }
}

/// The core application state structure used by the `eframe` (egui) integration.
///
/// This struct implements the `eframe::App` trait (in `update.rs`) and holds
/// all necessary state for rendering the UI and interacting with the backend engine.
pub struct TabletMapperApp {
    // Shared State
    /// Thread-safe state shared with the background engine thread.
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
    /// Send side of the async save channel. The background saver thread
    /// owns the receiver and writes `last_session.json` off the UI thread.
    pub save_sender: crossbeam_channel::Sender<MappingConfig>,

    // Toast Notifications
    /// Active toast queue. Max 3 simultaneous, deduplicated by message.
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
    /// Kept alive to prevent the tray icon from being dropped.
    pub tray_icon: Option<tray_icon::TrayIcon>,

    // Close Confirmation
    /// Whether the unsaved-changes close confirmation dialog is open.
    pub show_close_confirm: bool,
    /// If true, bypasses the close confirmation intercept.
    pub force_close: bool,
}

const MAX_TOASTS: usize = 3;

impl TabletMapperApp {
    /// Pushes a toast notification, deduplicating by message and capping at 3.
    pub fn push_toast(&mut self, message: String, level: ToastLevel) {
        // Deduplicate: don't push if an identical message is already visible
        if self.toasts.iter().any(|t| t.message == message) {
            return;
        }

        // Cap at MAX_TOASTS: remove oldest if full
        if self.toasts.len() >= MAX_TOASTS {
            self.toasts.remove(0);
        }

        self.toasts.push(Toast {
            message,
            level,
            created_at: Instant::now(),
        });
    }

    /// Loads a specific profile from a given path.
    pub fn load_profile_at_path(&mut self, path: PathBuf) {
        match crate::settings::load_settings_from_file(&path) {
            Ok((cfg, corrections)) => {
                self.apply_config(cfg.clone());

                // Update profile state
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
                    self.push_toast(
                        format!(
                            "Config repaired: {} field(s) reset to defaults",
                            corrections.len()
                        ),
                        ToastLevel::Warning,
                    );
                }
                self.push_toast(
                    format!("Loaded profile: {}", self.profile.name),
                    ToastLevel::Info,
                );
            }
            Err(e) => {
                self.push_toast(format!("Failed to load profile: {}", e), ToastLevel::Error);
            }
        }
    }

    /// Prompts the user to load a settings file and applies it.
    pub fn load_settings(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(crate::settings::get_settings_dir())
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            self.load_profile_at_path(path);
        }
    }

    /// Saves the current configuration to its associated file path, or prompts "Save As" if none exists.
    pub fn save_settings(&mut self, config: &MappingConfig) {
        let config = config.clone();
        if let Some(ref path) = self.profile.path {
            match crate::settings::save_to_path(path, &config) {
                Ok(()) => {
                    self.profile.mark_saved(&config);
                    if self.save_sender.try_send(config).is_err() {
                        log::warn!(target: "Config", "Save queue full, config change dropped");
                    }
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

    /// Prompts the user for a path and saves the configuration.
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
                    if self.save_sender.try_send(config).is_err() {
                        log::warn!(target: "Config", "Save queue full, config change dropped");
                    }

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

    /// Resets the current configuration to defaults while preserving system settings.
    pub fn reset_to_default(&mut self) {
        {
            let mut shared_config = self.shared.config.write().unwrap_or_log("config");
            let theme = shared_config.theme;
            let run_at_startup = shared_config.run_at_startup;

            *shared_config = MappingConfig::default();
            shared_config.theme = theme;
            shared_config.run_at_startup = run_at_startup;

            self.shared
                .config_version
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        self.push_toast(
            "Settings reset to default (Unsaved)".to_string(),
            ToastLevel::Info,
        );
    }

    /// Exports the configuration to an external file.
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

    /// Imports a configuration from an external file without changing the profile identity.
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
                            format!(
                                "Imported config repaired: {} field(s) reset",
                                corrections.len()
                            ),
                            ToastLevel::Warning,
                        );
                    }
                }
                Err(e) => self.push_toast(format!("Import failed: {}", e), ToastLevel::Error),
            }
        }
    }

    /// Filters the global log buffer based on current UI settings and search query.
    /// Returns (total_count, filtered_logs, full_log_text).
    pub fn get_filtered_logs(&mut self) -> (usize, &[crate::logger::LogEntry], &str) {
        let logs = crate::logger::LOG_BUFFER.read().unwrap_or_log("logs");
        
        let current_filters = (
            self.console_show_info,
            self.console_show_warn,
            self.console_show_error,
            self.console_show_debug,
        );

        if self.console_cache_log_count == logs.len()
            && self.console_cache_search == self.console_search
            && self.console_cache_filters == current_filters
        {
            return (
                self.console_cache_log_count,
                &self.console_cache_filtered,
                &self.console_cache_full_text,
            );
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
                log.message.to_lowercase().contains(&search_lower)
                    || log.group.to_lowercase().contains(&search_lower)
            })
            .cloned()
            .collect();

        filtered.reverse();

        let full_text = logs
            .iter()
            .map(|l| format!("[{}] {} [{}] {}", l.time, l.level, l.group, l.message))
            .collect::<Vec<_>>()
            .join("\n");

        self.console_cache_log_count = logs.len();
        self.console_cache_search = self.console_search.clone();
        self.console_cache_filters = current_filters;
        self.console_cache_filtered = filtered;
        self.console_cache_full_text = full_text;

        (
            self.console_cache_log_count,
            &self.console_cache_filtered,
            &self.console_cache_full_text,
        )
    }

    /// Initiates the download and installation of an available update in a background thread.
    pub fn start_update(&mut self) {
        if let crate::app::autoupdate::UpdateStatus::Available(release) = &self.update_status {
            let release_clone = release.clone();
            let sender = self.update_sender.clone();
            std::thread::spawn(move || {
                if let Err(e) =
                    crate::app::autoupdate::download_and_install(release_clone, sender.clone())
                {
                    let _ = sender.send(crate::app::autoupdate::UpdateStatus::Error(e.to_string()));
                }
            });
            self.update_status = crate::app::autoupdate::UpdateStatus::Downloading(0.0);
        }
    }

    /// Dismisses the update notification for the current session.
    pub fn dismiss_update(&mut self) {
        self.update_status = crate::app::autoupdate::UpdateStatus::Idle;
    }

    /// Triggers a manual check for updates from GitHub.
    pub fn check_for_updates(&mut self) {
        let sender = self.update_sender.clone();
        self.update_status = crate::app::autoupdate::UpdateStatus::Checking;
        std::thread::spawn(move || match crate::app::autoupdate::check_for_updates() {
            Ok(Some(release)) => {
                let _ = sender.send(crate::app::autoupdate::UpdateStatus::Available(release));
            }
            Ok(None) => {
                let _ = sender.send(crate::app::autoupdate::UpdateStatus::Idle);
            }
            Err(e) => {
                let _ = sender.send(crate::app::autoupdate::UpdateStatus::Error(e.to_string()));
            }
        });
    }

    /// Atomically updates the shared config and bumps the version counter.
    fn apply_config(&self, cfg: MappingConfig) { 
        {
            let mut shared_config = self.shared.config.write().unwrap_or_log("config");
            *shared_config = cfg.clone();
            self.shared
                .config_version
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        let _ = self.save_sender.try_send(cfg);
    }
}
