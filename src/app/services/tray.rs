use eframe::egui::Context;
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent, MouseButton};

pub struct TrayService {
    pub tray_icon: Option<TrayIcon>,
}

impl TrayService {
    pub fn new(ctx: Context) -> Self {
        let icon_bytes = include_bytes!("../../../resources/icon.png");
        let image = image::load_from_memory(icon_bytes)
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let icon = tray_icon::Icon::from_rgba(image.into_raw(), width, height)
            .expect("Failed to create tray icon from RGBA data");

        let tray_icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("NextTabletDriver")
            .build()
            .ok();

        if tray_icon.is_some() {
            let tray_ctx = ctx.clone();
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

                        #[cfg(windows)]
                        unsafe {
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
                            let hwnd = FindWindowA(std::ptr::null(), title.as_ptr() as *const _);
                            if hwnd != 0 {
                                log::info!(target: "Tray", "Native window found (HWND: {}), restoring...", hwnd);
                                ShowWindow(hwnd, 9); // SW_RESTORE
                                SetForegroundWindow(hwnd);
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
