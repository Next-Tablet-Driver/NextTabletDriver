//! # Application Lifecycle
//!
//! This module handles the initialization, startup routines, and background thread
//! management for the `TabletMapperApp`.

use display_info::DisplayInfo;
use std::sync::Arc;
use std::time::Instant;

use crate::app::autoupdate::UpdateStatus;
use crate::app::state::{AppTab, Metrics, ProfileState, TabletMapperApp, ToastLevel};
use crate::engine::state::SharedState;
use crate::settings::load_session_meta;

impl TabletMapperApp {
    /// Creates a new instance of the application with pre-initialized shared state and channels.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ctx: &eframe::egui::Context,
        shared: Arc<SharedState>,
        config: crate::core::config::models::MappingConfig,
        load_corrections: &[String],
        tablet_receiver: crossbeam_channel::Receiver<crate::drivers::TabletData>,
        update_receiver: crossbeam_channel::Receiver<UpdateStatus>,
        update_sender: crossbeam_channel::Sender<UpdateStatus>,
        save_sender: crossbeam_channel::Sender<crate::core::config::models::MappingConfig>,
    ) -> Self {
        // SAFETY: These are standard Windows API calls to set the process priority
        // and timer resolution for high-performance tablet input.
        #[cfg(windows)]
        {
            use windows_sys::Win32::Media::timeBeginPeriod;
            use windows_sys::Win32::System::Threading::{
                GetCurrentProcess, HIGH_PRIORITY_CLASS, SetPriorityClass,
            };
            // SAFETY: Setting timer resolution to 1ms for low-latency tablet polling.
            unsafe { timeBeginPeriod(1) };
            // SAFETY: Retrieving the handle to the current process.
            let process = unsafe { GetCurrentProcess() };
            // SAFETY: Increasing process priority to HIGH for stable driver performance.
            unsafe { SetPriorityClass(process, HIGH_PRIORITY_CLASS) };
        }

        // 1. Setup UI Appearance
        let mut app_prefs = crate::settings::app_preferences::load_app_preferences();
        crate::settings::app_preferences::validate_theme(&mut app_prefs);
        crate::ui::theme::apply_theme(ctx, &app_prefs.theme);
        Self::setup_fonts(ctx);

        // 2. Build initial state
        let mut initial_toasts = Vec::new();
        if !load_corrections.is_empty() {
            initial_toasts.push(crate::app::state::Toast {
                message: format!(
                    "Config repaired: {} field(s) reset to defaults",
                    load_corrections.len()
                ),
                level: ToastLevel::Warning,
                created_at: Instant::now(),
            });
        }

        let meta = load_session_meta();

        Self {
            shared,
            displays: DisplayInfo::all().unwrap_or_default(),
            last_update: Instant::now(),
            last_config_log: Instant::now(),
            profile: ProfileState {
                name: meta
                    .as_ref()
                    .map_or_else(|| "Unsaved Session".to_string(), |m| m.profile_name.clone()),
                path: meta.and_then(|m| m.profile_path),
                last_saved: config,
            },
            active_tab: AppTab::Output,
            tablet_receiver,
            update_receiver,
            update_sender,
            update_status: UpdateStatus::Idle,
            save_sender,
            toasts: initial_toasts,
            selected_filter: "Devocub Antichatter".to_string(),
            show_debugger: false,
            show_latency_stats: false,
            metrics: Metrics::default(),
            was_minimized: false,
            console_search: String::new(),
            console_show_info: true,
            console_show_warn: true,
            console_show_error: true,
            console_show_debug: true,
            console_autoscroll: true,
            console_cache_log_sequence: 0,
            console_cache_search: String::new(),
            console_cache_filters: (true, true, true, true),
            console_cache_filtered: Vec::new(),
            theme_store_open: false,
            theme_store_loading: false,
            theme_store_list: std::sync::Arc::new(std::sync::Mutex::new(None)),
            theme_store_search: String::new(),
            theme_store_filter_mode: None,
            show_close_confirm: false,
            force_close: false,
            missing_udev_rules: {
                #[cfg(target_os = "linux")]
                {
                    !std::path::Path::new("/etc/udev/rules.d/99-nexttabletdriver.rules").exists()
                        && !std::path::Path::new("/usr/lib/udev/rules.d/99-nexttabletdriver.rules")
                            .exists()
                }
                #[cfg(not(target_os = "linux"))]
                {
                    false
                }
            },
            app_prefs,
        }
    }

    fn setup_fonts(ctx: &eframe::egui::Context) {
        let mut fonts = eframe::egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        fonts.font_data.insert(
            "Helvetica".to_owned(),
            std::sync::Arc::new(eframe::egui::FontData::from_static(include_bytes!(
                "../../resources/fonts/Helvetica.ttf"
            ))),
        );
        fonts
            .families
            .entry(eframe::egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Helvetica".to_owned());
        ctx.set_fonts(fonts);
    }
}
