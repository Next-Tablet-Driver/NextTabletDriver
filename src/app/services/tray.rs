use crate::engine::state::SharedState;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

// System tray service
// - Minimal platform-specific code (Windows restore)
// - Decoupled icon loading
// - Clean event matching helper
// - Detached listener thread that processes tray & menu events

pub struct TrayService {
    pub tray_icon: Option<TrayIcon>,
}

impl TrayService {
    #[must_use]
    pub fn new(shared: &Arc<SharedState>) -> Self {
        let shared_clone = Arc::clone(shared);
        std::thread::spawn(move || Self::tray_event_loop(&shared_clone));
        Self { tray_icon: None }
    }

    fn load_icon() -> Option<tray_icon::Icon> {
        let icon_bytes = include_bytes!("../../../resources/icon.png");
        match image::load_from_memory(icon_bytes) {
            Ok(img) => {
                let image = img.into_rgba8();
                let (width, height) = image.dimensions();
                match tray_icon::Icon::from_rgba(image.into_raw(), width, height) {
                    Ok(icon) => Some(icon),
                    Err(e) => {
                        log::error!(target: "Tray", "Failed to create tray icon: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                log::error!(target: "Tray", "Failed to load tray icon image: {e}");
                None
            }
        }
    }

    const fn is_restore_event(event: &TrayIconEvent) -> bool {
        // React on left button release (Up) or on double click. This avoids
        // acting on the initial button-down event which may happen before
        // the system performs other actions.
        matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            } | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            }
        )
    }

    fn tray_event_loop(shared: &SharedState) {
        #[cfg(target_os = "linux")]
        {
            if let Err(e) = gtk::init() {
                log::error!(target: "Tray", "Failed to initialize GTK: {e:?}");
                return;
            }
        }

        let status_item = MenuItem::with_id("status", "Disconnected", false, None);
        let reload_item = MenuItem::with_id("reload", "Restart Driver", true, None);
        let quit_item = MenuItem::with_id("quit", "Exit", true, None);

        // Create the TrayIcon ON THIS THREAD so it owns the hidden message HWND
        let _tray_icon = Self::load_icon().and_then(|icon| {
            let menu = {
                let menu = Menu::new();
                if let Err(e) = menu.append(&status_item) {
                    log::error!(target: "Tray", "Failed to append status item: {e:?}");
                }
                if let Err(e) = menu.append(&PredefinedMenuItem::separator()) {
                    log::error!(target: "Tray", "Failed to append separator: {e:?}");
                }
                if let Err(e) = menu.append(&reload_item) {
                    log::error!(target: "Tray", "Failed to append reload item: {e:?}");
                }
                if let Err(e) = menu.append(&quit_item) {
                    log::error!(target: "Tray", "Failed to append quit item: {e:?}");
                }
                menu
            };

            TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip("NextTabletDriver")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .with_menu_on_right_click(true)
                .build()
                .map_err(|e| {
                    log::error!(target: "Tray", "Failed to build tray icon: {e:?}");
                    e
                })
                .ok()
        });

        // Receivers for tray and menu events
        let tray_receiver = TrayIconEvent::receiver().clone();
        let menu_receiver = tray_icon::menu::MenuEvent::receiver().clone();

        log::info!(target: "Tray", "System Tray listener background thread started");

        #[cfg(windows)]
        {
            use crate::engine::state::LockRecoveryExt;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
            };
            // SAFETY: MSG struct can be zeroed safely
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            let mut last_device_name = String::new();

            loop {
                // Process all pending messages
                // SAFETY: msg is valid, hwnd is null, filter min/max are 0, PM_REMOVE removes processed messages
                while unsafe { PeekMessageW(&raw mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) }
                    > 0
                {
                    if msg.message == WM_QUIT {
                        break;
                    }
                    // SAFETY: msg was properly populated by PeekMessageW
                    unsafe { TranslateMessage(&raw const msg) };
                    // SAFETY: msg was properly populated by PeekMessageW
                    unsafe { DispatchMessageW(&raw const msg) };
                }

                // Update status text when user interacts with tray
                let device = shared.device.read().unwrap_or_log("device");
                let current_name = device.name.clone();
                drop(device);

                if current_name != last_device_name {
                    if current_name.is_empty() {
                        status_item.set_text("Disconnected");
                    } else {
                        status_item.set_text(&current_name);
                    }
                    last_device_name = current_name;
                }

                // After dispatching, check if tray_icon generated any events
                while let Ok(event) = tray_receiver.try_recv() {
                    if Self::is_restore_event(&event) {
                        log::info!(target: "Tray", "Received Tray Event: {event:?}");
                        log::info!(target: "Tray", "Restoring UI from tray...");
                        shared.lifecycle.is_visible.store(true, Ordering::Release);
                    } else {
                        log::trace!(target: "Tray", "Tray Event: {event:?}");
                    }
                }

                while let Ok(menu_event) = menu_receiver.try_recv() {
                    log::info!(target: "Tray", "Menu event: {menu_event:?}");
                    match menu_event.id().0.as_str() {
                        "quit" => {
                            log::info!(target: "Tray", "Quit requested from tray menu");
                            crate::app::telemetry::capture_app_closed(shared);
                            std::process::exit(0);
                        }
                        "reload" => {
                            log::info!(target: "Tray", "Engine reload requested from tray menu");
                            shared
                                .config
                                .reload_requested
                                .store(true, Ordering::Release);
                        }
                        _ => {}
                    }
                }

                if shared.lifecycle.shutdown_requested.load(Ordering::Relaxed) {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }

        #[cfg(not(windows))]
        {
            use crate::engine::state::LockRecoveryExt;
            let mut last_device_name = String::new();
            loop {
                // Update status text
                let device = shared.device.read().unwrap_or_log("device");
                let current_name = device.name.clone();
                drop(device);

                if current_name != last_device_name {
                    if current_name.is_empty() {
                        status_item.set_text("Disconnected");
                    } else {
                        status_item.set_text(&current_name);
                    }
                    last_device_name = current_name;
                }

                #[cfg(target_os = "linux")]
                {
                    while gtk::events_pending() {
                        gtk::main_iteration();
                    }
                }

                let mut disconnected = false;
                loop {
                    match tray_receiver.try_recv() {
                        Ok(event) => {
                            if Self::is_restore_event(&event) {
                                log::info!(target: "Tray", "Received Tray Event: {event:?}");
                                log::info!(target: "Tray", "Restoring UI from tray...");
                                shared.lifecycle.is_visible.store(true, Ordering::Release);
                            } else {
                                log::trace!(target: "Tray", "Tray Event: {event:?}");
                            }
                        }
                        Err(crossbeam_channel::TryRecvError::Empty) => break,
                        Err(crossbeam_channel::TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }

                loop {
                    match menu_receiver.try_recv() {
                        Ok(menu_event) => {
                            log::info!(target: "Tray", "Menu event: {menu_event:?}");
                            match menu_event.id().0.as_str() {
                                "quit" => {
                                    log::info!(target: "Tray", "Quit requested from tray menu");
                                    std::process::exit(0);
                                }
                                "reload" => {
                                    log::info!(target: "Tray", "Engine reload requested from tray menu");
                                    shared
                                        .config
                                        .reload_requested
                                        .store(true, Ordering::Release);
                                }
                                _ => {}
                            }
                        }
                        Err(crossbeam_channel::TryRecvError::Empty) => break,
                        Err(crossbeam_channel::TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }

                if disconnected {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }

        log::info!(target: "Tray", "System Tray listener background thread exiting");
    }
}
