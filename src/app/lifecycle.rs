//! # Application Lifecycle
//!
//! This module handles the initialization, startup routines, and background thread
//! management for the `TabletMapperApp`.

use display_info::DisplayInfo;
use std::sync::Arc;
use std::time::Instant;
use crossbeam_channel::unbounded;

use crate::app::autoupdate::UpdateStatus;
use crate::app::state::{AppTab, ProfileState, TabletMapperApp, ToastLevel, Metrics};
use crate::settings::load_session_meta;
use crate::app::services::{ConfigService, UpdateService, TrayService, SharedStateFactory, ThreadSupervisor};

impl TabletMapperApp {
    /// Creates a new instance of the application and initializes all background services.
    pub fn new(ctx: eframe::egui::Context) -> Self {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Media::timeBeginPeriod;
            use windows_sys::Win32::System::Threading::{
                GetCurrentProcess, HIGH_PRIORITY_CLASS, SetPriorityClass,
            };
            timeBeginPeriod(1);
            SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
        }

        // 1. Load Configuration
        let config_service = ConfigService::load();
        let config = config_service.config;
        let load_corrections = config_service.corrections;
        let is_first_run = load_corrections.is_empty() && !crate::settings::get_settings_dir().exists();

        // 2. Setup UI Appearance
        crate::ui::theme::apply_theme(&ctx, config.theme);
        Self::setup_fonts(&ctx);

        // 3. Initialize Shared State
        let shared = SharedStateFactory::create(config.clone(), is_first_run);

        // 4. Initialize Services and Channels
        let (tablet_sender, tablet_receiver) = unbounded();
        let update_service = UpdateService::new();
        let (save_sender, save_receiver) = crossbeam_channel::bounded(1);
        let tray_service = TrayService::new(ctx.clone());

        // 5. Spawn Background Threads via Supervisor
        ThreadSupervisor::spawn_engine(Arc::clone(&shared), ctx.clone(), tablet_sender);
        ThreadSupervisor::spawn_websocket(Arc::clone(&shared));
        ThreadSupervisor::spawn_saver(save_receiver);
        update_service.start_check();

        // 6. Build initial state
        let mut initial_toasts = Vec::new();
        if !load_corrections.is_empty() {
            initial_toasts.push(crate::app::state::Toast {
                message: format!("Config repaired: {} field(s) reset to defaults", load_corrections.len()),
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
                name: meta.as_ref().map_or_else(|| "Unsaved Session".to_string(), |m| m.profile_name.clone()),
                path: meta.and_then(|m| m.profile_path),
                last_saved: config,
            },
            active_tab: AppTab::Output,
            tablet_receiver,
            update_receiver: update_service.receiver,
            update_sender: update_service.sender,
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
            console_cache_log_count: 0,
            console_cache_search: String::new(),
            console_cache_filters: (true, true, true, true),
            console_cache_filtered: Vec::new(),
            console_cache_full_text: String::new(),
            tray_icon: tray_service.tray_icon,
            show_close_confirm: false,
            force_close: false,
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
        fonts.families.entry(eframe::egui::FontFamily::Proportional).or_default().insert(0, "Helvetica".to_owned());
        ctx.set_fonts(fonts);
    }
}
