//! # Application Lifecycle
//!
//! This module handles the initialization, startup routines, and background thread
//! management for the `TabletMapperApp`. It is responsible for loading configurations
//! and bootstrapping the various concurrent systems (engine, websocket, updater).

use display_info::DisplayInfo;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicU32;
use std::thread;
use std::time::Instant;

use crate::app::autoupdate::{self, UpdateStatus};
use crate::app::state::{AppTab, ProfileState, TabletMapperApp, ToastLevel};
use crate::core::config::models::MappingConfig;
use crate::drivers::TabletData;
use crate::engine::state::SharedState;
use crate::engine::tablet_manager::run_manager;
use crate::settings::{load_last_session, load_session_meta};

impl TabletMapperApp {
    /// Creates a new instance of the application and initializes all background services.
    ///
    /// # Arguments
    /// * `_ctx` - The `eframe::egui::Context` provided by the window manager.
    ///
    /// # Initialization Flow
    /// 1. **Displays**: Gathers information about connected monitors.
    /// 2. **Configuration**: Attempts to load the last session's settings from disk.
    ///    If no configuration is found, applies fallback defaults.
    /// 3. **Shared State**: Allocates the `SharedState` structure wrapped in an `Arc`.
    /// 4. **Input Engine Thread**: Spawns a background thread to run the `tablet_manager`,
    ///    which continuously polls USB/HID devices.
    /// 5. **WebSocket Thread**: Spawns a background thread to manage WebSocket connections
    ///    for external integrations (e.g., streaming overlays).
    /// 6. **Auto-Updater Thread**: Spawns a background thread to check GitHub releases
    ///    for newer versions of the software.
    /// 7. **Background Saver Thread**: Spawns a thread to write `last_session.json`
    ///    asynchronously, keeping disk I/O off the UI thread.
    pub fn new(_ctx: eframe::egui::Context) -> Self {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Media::timeBeginPeriod;
            use windows_sys::Win32::System::Threading::{
                GetCurrentProcess, HIGH_PRIORITY_CLASS, SetPriorityClass,
            };
            timeBeginPeriod(1);
            SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
        }

        let displays = DisplayInfo::all().unwrap_or_default();

        let loaded = load_last_session();
        let is_first_run = loaded.is_none();

        let (config, load_corrections) = if let Some((cfg, corrections)) = loaded {
            log::info!(target: "Config", "Using loaded configuration from last session");
            (cfg, corrections)
        } else {
            let cfg = MappingConfig {
                run_at_startup: crate::startup::is_run_at_startup_registered(),
                ..Default::default()
            };
            (cfg, Vec::new())
        };

        crate::ui::theme::apply_theme(&_ctx, config.theme);

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

        _ctx.set_fonts(fonts);

        let shared = Arc::new(SharedState {
            config: RwLock::new(config.clone()),
            config_version: AtomicU32::new(0),
            tablet_data: RwLock::new(TabletData::default()),
            device_state: RwLock::new(crate::engine::state::DeviceState::default()),
            is_first_run: RwLock::new(is_first_run),
            packet_count: AtomicU32::new(0),
            stats: RwLock::new(crate::drivers::DriverStats::default())
        });

        let (tablet_sender, tablet_receiver) = crossbeam_channel::unbounded();

        let thread_shared = Arc::clone(&shared);
        let thread_ctx = _ctx.clone();
        log::info!(target: "App", "Spawning Input Engine thread");
        thread::spawn(move || {
            run_manager(thread_shared, thread_ctx, tablet_sender);
        });

        let ws_shared = Arc::clone(&shared);
        log::info!(target: "WebSocket", "Spawning WebSocket thread");
        thread::spawn(move || {
            crate::app::websocket::websocket_loop(ws_shared);
        });

        let (update_sender, update_receiver) = crossbeam_channel::bounded(1);
        let sender = update_sender.clone();
        log::info!(target: "App", "Spawning Auto-Updater thread");
        thread::spawn(move || match autoupdate::check_for_updates() {
            Ok(Some(release)) => {
                let _ = sender.send(UpdateStatus::Available(release));
            }
            Ok(None) => {}
            Err(e) => {
                log::error!(target: "Update", "Failed to check for updates: {}", e);
            }
        });

        // Background saver: receives configs from the UI thread and writes
        // last_session.json without blocking the render loop.
        let (save_sender, save_receiver) = crossbeam_channel::bounded::<MappingConfig>(1);
        log::info!(target: "Config", "Spawning Background Saver thread");
        thread::spawn(move || {
            while let Ok(cfg) = save_receiver.recv() {
                // Drain any queued updates, keep only the latest
                let mut latest = cfg;
                while let Ok(newer) = save_receiver.try_recv() {
                    latest = newer;
                }
                if let Err(e) = crate::settings::save_last_session(&latest) {
                    log::error!(target: "Config", "Background saver failed: {}", e);
                }
            }
            log::error!(target: "Config", "Background saver thread exited");
        });

        let icon_bytes = include_bytes!("../../resources/icon.png");
        let image = image::load_from_memory(icon_bytes)
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let icon = tray_icon::Icon::from_rgba(image.into_raw(), width, height)
            .expect("Failed to create tray icon from RGBA data");

        let tray_icon = tray_icon::TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("NextTabletDriver")
            .build()
            .ok();

        let tray_ctx = _ctx.clone();
        thread::spawn(move || {
            let receiver = tray_icon::TrayIconEvent::receiver();
            log::info!(target: "Tray", "System Tray listener background thread started");
            while let Ok(event) = receiver.recv() {
                log::info!(target: "Tray", "Received Tray Event: {:?}", event);

                let matches = matches!(
                    event,
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        ..
                    } | tray_icon::TrayIconEvent::DoubleClick {
                        button: tray_icon::MouseButton::Left,
                        ..
                    }
                );

                if matches {
                    log::info!(target: "Tray", "Restoring eframe UI...");

                    #[cfg(windows)]
                    {
                        #[link(name = "user32")]
                        unsafe extern "system" {
                            fn FindWindowA(
                                lpClassName: *const std::ffi::c_char,
                                lpWindowName: *const std::ffi::c_char,
                            ) -> isize;
                            fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
                            fn SetForegroundWindow(hWnd: isize) -> i32;
                        }
                        unsafe {
                            let title = format!("NextTabletDriver v{}\0", crate::VERSION);
                            // Find our window by title for native Win32 restore
                            let hwnd = FindWindowA(std::ptr::null(), title.as_ptr() as *const _);
                            if hwnd != 0 {
                                log::info!(target: "Tray", "Native window found (HWND: {}), restoring...", hwnd);
                                ShowWindow(hwnd, 9); // SW_RESTORE
                                SetForegroundWindow(hwnd);
                            } else {
                                log::warn!(target: "Tray", "FindWindowA could not find target window title: {:?}", title);
                            }
                        }
                    }

                    // Sync eframe viewport state with the native window restore above
                    tray_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
                    tray_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
                    tray_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
                    tray_ctx.request_repaint();
                }
            }
            log::error!(target: "Tray", "System Tray listener thread died!");
        });

        // Build initial toast notifications for any config corrections
        let mut initial_toasts = Vec::new();
        if !load_corrections.is_empty() {
            let msg = format!(
                "Config repaired: {} field(s) had invalid values and were reset to defaults",
                load_corrections.len()
            );
            initial_toasts.push(crate::app::state::Toast {
                message: msg,
                level: ToastLevel::Warning,
                created_at: Instant::now(),
            });
        }

        Self {
            shared,
            displays,
            last_update: Instant::now(),
            last_config_log: Instant::now(),
            profile: {
                // Restore profile identity from session_meta.json (silent, no toast)
                let meta = load_session_meta();
                ProfileState {
                    name: meta
                        .as_ref()
                        .map_or_else(|| "Unsaved Session".to_string(), |m| m.profile_name.clone()),
                    path: meta.and_then(|m| m.profile_path),
                    last_saved: config.clone(),
                }
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
            metrics: crate::app::state::Metrics::default(),
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
            tray_icon,
            show_close_confirm: false,
            force_close: false,
        }
    }
}
