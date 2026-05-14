use eframe::egui::Context;
use tray_icon::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct TrayService {
    pub tray_icon: Option<TrayIcon>,
}

impl TrayService {
    pub fn new(ctx: Context) -> Self {
        let icon_bytes = include_bytes!("../../../resources/icon.png");
        let tray_icon = match image::load_from_memory(icon_bytes) {
            Ok(img) => {
                let image = img.into_rgba8();
                let (width, height) = image.dimensions();
                match tray_icon::Icon::from_rgba(image.into_raw(), width, height) {
                    Ok(icon) => TrayIconBuilder::new()
                        .with_icon(icon)
                        .with_tooltip("NextTabletDriver")
                        .build()
                        .ok(),
                    Err(e) => {
                        log::error!(target: "Tray", "Failed to create tray icon: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::error!(target: "Tray", "Failed to load tray icon image: {}", e);
                None
            }
        };

        if tray_icon.is_some() {
            let tray_ctx = ctx;
            std::thread::spawn(move || {
                let receiver = TrayIconEvent::receiver();
                log::info!(target: "Tray", "System Tray listener background thread started");
                while let Ok(event) = receiver.recv() {
                    log::info!(target: "Tray", "Received Tray Event: {:?}", event);

                    let matches = matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        } | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        }
                    );

                    if matches {
                        log::info!(target: "Tray", "Restoring eframe UI...");

                        // SAFETY: This uses native Windows APIs to find and restore the driver window
                        // when the tray icon is clicked.
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
                            let title = format!("NextTabletDriver v{}\0", crate::VERSION);
                            // SAFETY: Finding the window by its title.
                            let hwnd = unsafe {
                                FindWindowA(std::ptr::null(), title.as_ptr() as *const _)
                            };
                            if hwnd != 0 {
                                log::info!(target: "Tray", "Native window found (HWND: {}), restoring...", hwnd);
                                // SAFETY: Restoring the window to its normal state.
                                unsafe { ShowWindow(hwnd, 9) }; // SW_RESTORE
                                // SAFETY: Brining the window to the foreground.
                                unsafe { SetForegroundWindow(hwnd) };
                            }
                        }

                        tray_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
                        tray_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
                        tray_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
                        tray_ctx.request_repaint();
                    }
                }
            });
        }

        Self { tray_icon }
    }
}
