//! Desktop shell of `NextTabletDriver`: a Tauri program that starts the engine, shows the
//! interface,
//! and bridges the interface to the engine through commands and events.

use ntd_core::sync::LockRecoveryExt;
use tauri::Manager;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri_specta::Event;

mod commands;
pub mod events;

#[tauri::command]
#[specta::specta]
fn get_core_version() -> String {
    format!("v{}", next_tablet_driver::VERSION)
}

/// Every command and event the frontend can use. The TypeScript bindings are generated from this
/// list, so a command that is not here has no typed function on the frontend.
#[must_use]
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            get_core_version,
            commands::get_tablet_status,
            commands::get_metrics,
            commands::get_config,
            commands::set_config,
            commands::save_config,
            commands::reset_config,
            commands::normalize_binding,
            commands::load_profile,
            commands::import_otd_profile,
            commands::export_profile,
            commands::get_presets,
            commands::get_current_profile_name,
            commands::get_logs,
            commands::clear_logs,
            commands::get_releases,
            commands::get_available_plugins,
            commands::reload_plugins,
            commands::open_plugins_folder,
            commands::install_plugin,
            commands::get_untrusted_plugins,
            commands::trust_plugin,
            commands::delete_plugin,
            commands::open_themes_folder,
            commands::list_themes,
            commands::import_theme,
            commands::delete_theme
        ])
        .events(tauri_specta::collect_events![
            events::TabletEvent,
            events::LogEvent,
            events::DeviceChanged,
            events::ProfileLoaded,
            events::NavigateToUpdates
        ])
}

/// Starts the application: the logger, the tray icon, the engine threads and the Tauri event loop.
/// Returns when the application exits.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(
    clippy::print_stderr,
    reason = "when the application cannot start, the log may not be readable yet"
)]
pub fn run() {
    // Delay initialization on autostart to allow network and WebView2 to be ready
    if std::env::args().any(|arg| arg == "--autostart") {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    let _ = next_tablet_driver::logger::init();

    let specta_builder = specta_builder();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(specta_builder.invoke_handler())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state: tauri::State<
                    '_,
                    std::sync::Arc<next_tablet_driver::engine::state::SharedState>,
                > = window.state();
                let config = state
                    .config
                    .mapping
                    .read()
                    .unwrap_or_log("config_read_tauri");
                if config.system_tray_on_minimize {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(move |app| {
            specta_builder.mount_events(app);

            // Load Configuration early for Tray
            let config_service = next_tablet_driver::application::services::ConfigService::load();
            let config = config_service.config.clone();
            let is_first_run = config_service.corrections.is_empty()
                && !next_tablet_driver::settings::get_settings_dir().exists();

            // Setup System Tray
            let show_i = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;

            let status_i = MenuItem::with_id(app, "status", "Disconnected", false, None::<&str>)?;

            let profiles = next_tablet_driver::settings::list_profiles();
            let mut profile_items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = Vec::new();
            for (name, path) in profiles {
                let id = format!("profile|{}", path.to_string_lossy());
                let item = MenuItem::with_id(app, id, name, true, None::<&str>)?;
                profile_items.push(Box::new(item));
            }
            let profile_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = profile_items
                .iter()
                .map(|item| &**item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
                .collect();
            let profiles_menu = tauri::menu::Submenu::with_id_and_items(
                app,
                "profiles",
                "Profiles",
                true,
                &profile_refs,
            )?;

            let mode_absolute_i = CheckMenuItem::with_id(
                app,
                "mode_absolute",
                "Absolute",
                true,
                config.mode == ntd_core::config::models::DriverMode::Absolute,
                None::<&str>,
            )?;
            let mode_relative_i = CheckMenuItem::with_id(
                app,
                "mode_relative",
                "Relative",
                true,
                config.mode == ntd_core::config::models::DriverMode::Relative,
                None::<&str>,
            )?;
            let mode_menu = Submenu::with_id_and_items(
                app,
                "mode",
                "Mode",
                true,
                &[&mode_absolute_i, &mode_relative_i],
            )?;

            let plugins_i = MenuItem::with_id(
                app,
                "open_plugins",
                "Open Plugins \
            Folder",
                true,
                None::<&str>,
            )?;
            let themes_i = MenuItem::with_id(
                app,
                "open_themes",
                "Open Themes \
            Folder",
                true,
                None::<&str>,
            )?;

            let update_i = MenuItem::with_id(
                app,
                "check_updates",
                "Check for \
            Updates",
                true,
                None::<&str>,
            )?;

            let reload_i = MenuItem::with_id(app, "reload", "Reload Engine", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let sep1 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let sep2 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let sep3 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let sep4 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let sep5 = tauri::menu::PredefinedMenuItem::separator(app)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_i,
                    &sep1,
                    &status_i,
                    &profiles_menu,
                    &mode_menu,
                    &sep2,
                    &plugins_i,
                    &themes_i,
                    &sep3,
                    &update_i,
                    &sep4,
                    &reload_i,
                    &sep5,
                    &quit_i,
                ],
            )?;

            let mut tray_builder = TrayIconBuilder::new();
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _tray = tray_builder
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    if id == "quit" {
                        app.exit(0);
                    } else if id == "show" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if id == "reload" {
                        let state: tauri::State<
                            '_,
                            std::sync::Arc<next_tablet_driver::engine::state::SharedState>,
                        > = app.state();
                        state
                            .config
                            .reload_requested
                            .store(true, std::sync::atomic::Ordering::Release);
                    } else if id == "mode_absolute" || id == "mode_relative" {
                        let state: tauri::State<
                            '_,
                            std::sync::Arc<next_tablet_driver::engine::state::SharedState>,
                        > = app.state();
                        let mut mapping_config = state
                            .config
                            .mapping
                            .write()
                            .unwrap_or_log("config_write_tauri");
                        let new_mode = if id == "mode_absolute" {
                            ntd_core::config::models::DriverMode::Absolute
                        } else {
                            ntd_core::config::models::DriverMode::Relative
                        };

                        if mapping_config.mode != new_mode {
                            mapping_config.mode = new_mode;
                            let config_clone = mapping_config.clone();
                            drop(mapping_config);
                            state
                                .config
                                .reload_requested
                                .store(true, std::sync::atomic::Ordering::Release);

                            let save_sender: tauri::State<
                                '_,
                                crossbeam_channel::Sender<ntd_core::config::models::MappingConfig>,
                            > = app.state();
                            let _ = save_sender.try_send(config_clone);
                            let _ = events::ProfileLoaded.emit(app);
                        }
                    } else if id == "open_plugins" {
                        let state: tauri::State<
                            '_,
                            std::sync::Arc<next_tablet_driver::engine::state::SharedState>,
                        > = app.state();
                        let _ = crate::commands::open_plugins_folder(state);
                    } else if id == "open_themes" {
                        let _ = crate::commands::open_themes_folder();
                    } else if id == "check_updates" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = events::NavigateToUpdates.emit(app);
                        }
                    } else if id.starts_with("profile|") {
                        let path = id.trim_start_matches("profile|");
                        let state: tauri::State<
                            '_,
                            std::sync::Arc<next_tablet_driver::engine::state::SharedState>,
                        > = app.state();
                        if crate::commands::load_profile(state, path.to_string()).is_ok() {
                            log::info!(target: "Tray", "Loaded profile from tray: {path}");
                            let _ = events::ProfileLoaded.emit(app);
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Show window if not started automatically
            if !std::env::args().any(|arg| arg == "--autostart")
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.show();
                let _ = window.set_focus();
            }

            // Apply global OS settings from config
            next_tablet_driver::startup::set_fast_timer(config.force_high_resolution_timer);

            // Create Shared State
            let shared = next_tablet_driver::application::services::SharedStateFactory::create(
                config,
                is_first_run,
            );

            // Setup channels for threads
            let (tablet_sender, tablet_receiver) = crossbeam_channel::bounded(60);
            let (save_sender, save_receiver) = crossbeam_channel::bounded(1);

            // Start Background Threads (The "Engine")
            next_tablet_driver::application::services::ThreadSupervisor::spawn_engine(
                std::sync::Arc::clone(&shared),
                tablet_sender,
            );
            next_tablet_driver::application::services::ThreadSupervisor::spawn_websocket(
                std::sync::Arc::clone(&shared),
            );
            next_tablet_driver::application::services::ThreadSupervisor::spawn_saver(save_receiver);

            // Event Bridge: Pipe HID packets to Tauri Frontend
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_emit = std::time::Instant::now();
                let emit_interval = std::time::Duration::from_millis(16); // ~60Hz for UI

                for frame in tablet_receiver {
                    let now = std::time::Instant::now();
                    if now.duration_since(last_emit) >= emit_interval {
                        last_emit = now;

                        // If all windows are hidden (e.g. Minimized to System Tray), we
                        // completely skip JSON serialization and IPC emission. This gives
                        // 100% of CPU back to the driver engine.
                        let any_visible = app_handle
                            .webview_windows()
                            .values()
                            .any(|w| w.is_visible().unwrap_or(false));

                        if any_visible {
                            // Map the complex TabletData to a simple serializable struct for Svelte
                            let payload = events::TabletEvent {
                                status: frame.status.as_str().to_owned(),
                                x: frame.x,
                                y: frame.y,
                                pressure: frame.pressure,
                                hover_distance: frame.hover_distance,
                                tilt_x: frame.tilt_x,
                                tilt_y: frame.tilt_y,
                            };
                            let _ = payload.emit(&app_handle);
                        }
                    }
                }
            });

            let app_handle_status = app.handle().clone();
            let shared_status = std::sync::Arc::clone(&shared);
            let status_i_clone = status_i;

            std::thread::spawn(move || {
                let mut last_name = String::new();
                loop {
                    let current_name = match shared_status.device.read() {
                        Ok(guard) => guard.name.clone(),
                        Err(_) => break,
                    };

                    if current_name != last_name {
                        if current_name.is_empty() {
                            let _ = status_i_clone.set_text("Disconnected");
                        } else {
                            let _ = status_i_clone.set_text(&current_name);
                        }
                        last_name = current_name;
                        let _ = events::DeviceChanged.emit(&app_handle_status);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            });

            let mode_absolute_i_clone = mode_absolute_i;
            let mode_relative_i_clone = mode_relative_i;
            let shared_mode = std::sync::Arc::clone(&shared);

            std::thread::spawn(move || {
                let mut last_mode = shared_mode
                    .config
                    .mapping
                    .read()
                    .map_or(ntd_core::config::models::DriverMode::Absolute, |guard| {
                        guard.mode
                    });

                loop {
                    let current_mode = match shared_mode.config.mapping.read() {
                        Ok(guard) => guard.mode,
                        Err(_) => break,
                    };

                    if current_mode != last_mode {
                        let _ = mode_absolute_i_clone.set_checked(
                            current_mode == ntd_core::config::models::DriverMode::Absolute,
                        );
                        let _ = mode_relative_i_clone.set_checked(
                            current_mode == ntd_core::config::models::DriverMode::Relative,
                        );
                        last_mode = current_mode;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            });

            // Inject the State into Tauri so commands can access it
            app.manage(shared);
            app.manage(save_sender);

            log::info!(target: "Tauri", "NextTabletDriver Engine successfully booted in Tauri \
            context.");

            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(e) = result {
        log::error!(target: "Tauri", "Failed to run the Tauri application: {e}");
        eprintln!("Failed to run the Tauri application: {e}");
    }
}
