use eframe::egui::Context;
use tray_icon::menu::{Menu, MenuItem};
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
    pub fn new(ctx: &Context) -> Self {
        let tray_icon = Self::load_icon().and_then(|icon| {
            // Build a simple context menu with a "Quit" item.
            let menu = {
                let menu = Menu::new();
                let quit_item = MenuItem::with_id("quit", "Quit", true, None);
                if let Err(e) = menu.append(&quit_item) {
                    log::error!(target: "Tray", "Failed to append menu item: {e:?}");
                }
                menu
            };

            TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip("NextTabletDriver")
                // show menu only on right click, left click will be used to restore the window
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

        if tray_icon.is_some() {
            // Spawn a detached background thread to listen for tray/menu events.
            // This mirrors the original behaviour where the tray listener lives for the
            // application's lifetime. If you prefer graceful shutdown, consider
            // keeping the JoinHandle and joining on application exit.
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || Self::tray_event_loop(&ctx_clone));
        }

        Self { tray_icon }
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

    fn tray_event_loop(ctx: &Context) {
        // Receivers for tray and menu events
        let tray_receiver = TrayIconEvent::receiver().clone();
        let menu_receiver = tray_icon::menu::MenuEvent::receiver().clone();

        log::info!(target: "Tray", "System Tray listener background thread started");

        loop {
            crossbeam_channel::select! {
                recv(tray_receiver) -> res => match res {
                    Ok(event) => {
                        log::info!(target: "Tray", "Received Tray Event: {event:?}");

                        if Self::is_restore_event(&event) {
                            log::info!(target: "Tray", "Restoring eframe UI...");

                            // On Windows, try to restore the native window and bring it to foreground.
                            #[cfg(windows)]
                            {
                                use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowA, ShowWindow, SetForegroundWindow};
                                if let Ok(title_c) = std::ffi::CString::new(format!("NextTabletDriver v{}", crate::VERSION)) {
                                    // SAFETY: `title_c` is a valid null-terminated C string (created via `CString::new`).
                                    // Passing a null class name and a valid window name pointer to `FindWindowA` is safe.
                                    let hwnd = unsafe { FindWindowA(std::ptr::null(), title_c.as_ptr().cast()) };
                                    if !hwnd.is_null() {
                                        // SW_RESTORE = 9
                                        // SAFETY: `hwnd` was checked for non-null above, so it is assumed to be a valid window handle.
                                        unsafe { ShowWindow(hwnd, 9) };
                                        // SAFETY: `hwnd` was checked for non-null above, so it is assumed to be a valid window handle.
                                        unsafe { SetForegroundWindow(hwnd) };
                                    }
                                }
                            }

                            // Send eframe viewport commands to restore and focus the window.
                            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
                            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
                            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
                            ctx.request_repaint();
                        }
                    }
                    Err(_) => break,
                },
                recv(menu_receiver) -> res => match res {
                    Ok(menu_event) => {
                        log::info!(target: "Tray", "Menu event: {menu_event:?}");
                        if menu_event.id() == "quit" {
                            log::info!(target: "Tray", "Quit requested from tray menu");
                            // Terminate immediately from background thread. Replace with
                            // a channel to UI thread for graceful shutdown if needed.
                            std::process::exit(0);
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        log::info!(target: "Tray", "System Tray listener background thread exiting");
    }
}
