//! # `NextTabletDriver` Entry Point
//!
//! This is the main executable for the `NextTabletDriver` application.
//! It initializes logging, checks for single-instance enforcement,
//! configures the window properties, and launches the `eframe` (egui) graphical interface.

#![windows_subsystem = "windows"]

use eframe::egui;
use next_tablet_driver::app::TabletMapperApp;
use next_tablet_driver::app::services::{
    ConfigService, SharedStateFactory, ThreadSupervisor, TrayService, UpdateService,
};
use next_tablet_driver::logger;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

/// Ensures GPU renderer/vendor/version info is only logged and tracked once per
/// process, even though the main loop re-runs `eframe::run_native` (and its
/// startup closure) every time the window is restored from the system tray.
static GPU_INFO_REPORTED: AtomicBool = AtomicBool::new(false);

/// Adjusts the Windows system timer resolution to minimize input latency.
///
/// # Technical Details
/// By default, Windows uses a timer interval of ~15.6ms. For a high-performance
/// tablet driver, this can lead to "aliasing" or "jitter" where tablet reports
/// (often 1000Hz+) are processed in inconsistent batches.
///
/// This function calls the undocumented `NtSetTimerResolution` in `ntdll.dll`
/// to force a **0.5ms** (5000 units of 100ns) resolution, the maximum
/// precision supported by the Windows kernel.
///
/// # Arguments
/// * `enable` - `1` to request high precision, `0` to release the request.
#[cfg(windows)]
fn set_fast_timer(enable: u8) {
    // SAFETY: `GetModuleHandleA` is called with a valid static C string literal.
    let ntdll = unsafe { GetModuleHandleA(c"ntdll.dll".as_ptr().cast::<u8>()) };
    if ntdll.is_null() {
        log::warn!(target: "Timer", "Failed to get ntdll handle for timer resolution");
        return;
    }

    // SAFETY: `GetProcAddress` is called with a valid static C string function name on a verified ntdll handle.
    let addr_set = unsafe { GetProcAddress(ntdll, c"NtSetTimerResolution".as_ptr().cast::<u8>()) };

    // SAFETY: `GetProcAddress` is called with a valid static C string function name on a verified ntdll handle.
    let addr_query =
        unsafe { GetProcAddress(ntdll, c"NtQueryTimerResolution".as_ptr().cast::<u8>()) };

    if let (Some(addr_set), Some(addr_query)) = (addr_set, addr_query) {
        // SAFETY: The signature matches the documented prototype of `NtSetTimerResolution`.
        // Including `unsafe` in the type signature ensures the call itself is correctly flagged as unsafe.
        let nt_set: unsafe extern "system" fn(u32, u8, *mut u32) -> i32 =
            unsafe { std::mem::transmute(addr_set) };

        // SAFETY: The signature matches the documented prototype of `NtQueryTimerResolution`.
        // Including `unsafe` in the type signature ensures the call itself is correctly flagged as unsafe.
        let nt_query: unsafe extern "system" fn(*mut u32, *mut u32, *mut u32) -> i32 =
            unsafe { std::mem::transmute(addr_query) };

        let mut min = 0;
        let mut max = 0;
        let mut cur = 0;

        // SAFETY: All `&raw mut` pointers passed to NT functions point to valid, correctly aligned local variables.
        // This `unsafe` block is strictly required by the compiler because of the function pointer type.
        let _ = unsafe { nt_query(&raw mut min, &raw mut max, &raw mut cur) };

        log::debug!(target: "Timer", "System Timer Resolution: Min={:.1}ms, Max={:.1}ms, Current={:.1}ms",
            f64::from(min) / 10000.0, f64::from(max) / 10000.0, f64::from(cur) / 10000.0);

        // SAFETY: `timeBeginPeriod` is called with a valid parameter.
        unsafe { windows_sys::Win32::Media::timeBeginPeriod(1) };

        let mut new_cur = 0;

        // SAFETY: All `&raw mut` pointers passed to NT functions point to valid, correctly aligned local variables.
        // This `unsafe` block is strictly required by the compiler because of the function pointer type.
        let status = unsafe { nt_set(max, enable, &raw mut new_cur) };

        if status == 0 {
            log::info!(target: "Timer", "Timer resolution adjusted to {:.1}ms", f64::from(new_cur) / 10000.0);
        } else {
            log::warn!(target: "Timer", "Failed to adjust timer resolution (NTSTATUS: 0x{status:08X})");
        }
    } else {
        log::warn!(target: "Timer", "Could not find timer resolution functions in ntdll.dll");
    }
}

/// The main entry point of the application.
///
/// # Platform Specifics
/// - **Windows**: Creates a named Mutex to ensure only one instance is running.
///   This mutex is also checked by the Inno Setup installer.
/// - **Linux**: Uses a file lock in `$XDG_RUNTIME_DIR` (or `/tmp`) for single-instance enforcement.
///
/// # Execution Flow
/// Verifies no other instance is running, initializes the application logger,
/// configures the GUI window options (icon, dimensions, title), and enters
/// the `eframe::run_native` GUI event loop.
fn main() -> eframe::Result {
    next_tablet_driver::app::telemetry::setup_panic_hook();

    let startup_start = std::time::Instant::now();
    let logger_start = std::time::Instant::now();

    if let Err(e) = logger::init() {
        eprintln!("CRITICAL: Failed to initialize logger: {e}");
        std::process::exit(1);
    }
    let logger_duration = logger_start.elapsed();

    let mutex_start = std::time::Instant::now();
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        log::debug!(target: "Startup", "Checking for single instance (Windows Mutex)...");
        let mutex_name: Vec<u16> = "NextTabletDriverMutex\0".encode_utf16().collect();
        // SAFETY: `mutex_name` is a valid null-terminated wide string pointer. `null()` is valid
        // for the optional `lpMutexAttributes` parameter. The returned handle is checked below.
        let handle: HANDLE = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
        if handle.is_null() {
            log::error!(target: "Startup", "Failed to create mutex handle");
            return Ok(());
        }
        // SAFETY: called immediately after CreateMutexW on the same thread; the Win32 last error
        // code is valid and unmodified at this point.
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            log::error!(target: "Startup", "Another instance of NextTabletDriver is already running.");
            return Ok(());
        }

        set_fast_timer(1);
    }

    #[cfg(target_os = "linux")]
    let _lock_file = {
        use std::fs;
        use std::io::Write;

        log::debug!(target: "Startup", "Checking for single instance (Linux flock)...");

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        let lock_path = std::path::PathBuf::from(runtime_dir).join("nexttabletdriver.lock");

        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path);

        match file {
            Ok(mut f) => {
                use std::os::unix::io::AsRawFd;

                let fd = f.as_raw_fd();
                // SAFETY: `fd` is a valid file descriptor obtained from the open file `f`.
                // The lock is non-blocking (LOCK_NB) and exclusive (LOCK_EX).
                let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
                if ret != 0 {
                    log::error!(target: "Startup", "Another instance of NextTabletDriver is already running (PID locked).");
                    std::process::exit(1);
                }

                let _ = write!(f, "{}", std::process::id());
                Some(f) // Keeps flock alive for the process lifetime
            }
            Err(e) => {
                log::warn!(target: "Startup", "Could not create lock file at {}: {e}", lock_path.display());
                None
            }
        }
    };
    let mutex_duration = mutex_start.elapsed();

    log::info!(target: "Startup", "NextTabletDriver v{} starting on {} ({})",
        next_tablet_driver::VERSION,
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    let icon_data = eframe::icon_data::from_png_bytes(include_bytes!("../resources/icon.png"))
        .unwrap_or_else(|e| {
            log::warn!(target: "Startup", "Failed to load icon: {e}");
            egui::IconData::default()
        });

    // Migrate profiles if necessary
    next_tablet_driver::settings::migrate_profiles_to_subdir();

    // Load configuration
    let config_start = std::time::Instant::now();
    let config_service = ConfigService::load();
    let config = config_service.config.clone();
    let load_corrections = config_service.corrections;
    let is_first_run =
        load_corrections.is_empty() && !next_tablet_driver::settings::get_settings_dir().exists();
    let config_duration = config_start.elapsed();

    // Initialize shared state and I18N
    let state_start = std::time::Instant::now();
    let shared = SharedStateFactory::create(config.clone(), is_first_run);
    let app_prefs = next_tablet_driver::settings::app_preferences::load_app_preferences();

    next_tablet_driver::app::telemetry::TelemetryService::init(
        app_prefs.telemetry_id.clone(),
        app_prefs.telemetry_enabled,
    );

    next_tablet_driver::app::telemetry::send_pending_crash_reports();
    next_tablet_driver::i18n::set_locale(app_prefs.language);
    let total_ram_gb = next_tablet_driver::startup::get_memory_info()
        .map(|b| (b as f64 / 1_073_741_824.0).ceil() as u64);
    let state_duration = state_start.elapsed();

    // Initialize services and channels
    let services_start = std::time::Instant::now();
    let (tablet_sender, tablet_receiver) = crossbeam_channel::bounded(60);
    let update_service = UpdateService::new();
    let update_receiver = update_service.receiver.clone();
    let update_sender = update_service.sender.clone();
    let (save_sender, save_receiver) = crossbeam_channel::bounded(1);
    let _tray_service = TrayService::new(&shared);
    let services_duration = services_start.elapsed();

    // Spawn background threads via the supervisor
    let supervisor_start = std::time::Instant::now();
    ThreadSupervisor::spawn_engine(Arc::clone(&shared), tablet_sender);
    ThreadSupervisor::spawn_websocket(Arc::clone(&shared));
    ThreadSupervisor::spawn_saver(save_receiver);
    update_service.start_check();
    let supervisor_duration = supervisor_start.elapsed();

    // Log tracking info and startup timeline
    next_tablet_driver::startup::log_system_hardware();
    next_tablet_driver::settings::log_mapping_config(&config, "Startup");

    log::info!(
        target: "Startup",
        "TIMING - Logger: {:.2?} | Mutex/SingleInst: {:.2?} | Config Load: {:.2?} | Shared State: {:.2?} | Services/Tray: {:.2?} | Supervisor Threads: {:.2?} | Total Startup: {:.2?}",
        logger_duration,
        mutex_duration,
        config_duration,
        state_duration,
        services_duration,
        supervisor_duration,
        startup_start.elapsed()
    );

    let cpu_cores = std::env::var("NUMBER_OF_PROCESSORS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok());

    let theme_name = format!("{:?}", app_prefs.theme);

    next_tablet_driver::app::telemetry::capture_event_with_set(
        "app_started",
        Some(serde_json::json!({
            "startup_time_ms": startup_start.elapsed().as_millis(),
        })),
        Some(serde_json::json!({
            "language": app_prefs.language.display_name().to_string(),
            "cpu_cores": cpu_cores,
            "total_ram_gb": total_ram_gb,
            "driver_mode": format!("{:?}", config.mode),
            "current_theme": theme_name,
        })),
    );

    // Main app loop
    // eframe blocks until the window is closed. When minimized to tray, the window closes
    // and eframe returns. We sleep until the tray restores the window, then restart eframe.
    loop {
        if shared.lifecycle.shutdown_requested.load(Ordering::Relaxed) {
            log::info!(target: "App", "Shutdown requested, exiting main loop.");
            next_tablet_driver::app::telemetry::capture_app_closed(&shared);
            break Ok(());
        }

        if shared.lifecycle.is_visible.load(Ordering::Acquire) {
            let primary_display = display_info::DisplayInfo::all()
                .unwrap_or_default()
                .into_iter()
                .find(|d| d.is_primary);
            let mut viewport = egui::ViewportBuilder::default()
                .with_icon(icon_data.clone())
                .with_inner_size([1000.0, 850.0])
                .with_title(format!("NextTabletDriver v{}", next_tablet_driver::VERSION))
                .with_active(true);

            if let Some(d) = primary_display {
                let x = d.x as f32 + (d.width as f32 - 1000.0) / 2.0;
                let y = d.y as f32 + (d.height as f32 - 850.0) / 2.0;
                viewport = viewport.with_position(egui::pos2(x, y));
            }

            let options = eframe::NativeOptions {
                viewport,
                ..Default::default()
            };

            let ctx_shared = Arc::clone(&shared);
            let ctx_config = config.clone();
            let ctx_corrections = load_corrections.clone();
            let ctx_tablet_rx = tablet_receiver.clone();
            let ctx_update_rx = update_receiver.clone();
            let ctx_update_tx = update_sender.clone();
            let ctx_save_tx = save_sender.clone();

            let result = eframe::run_native(
                &format!("NextTabletDriver v{}", next_tablet_driver::VERSION),
                options,
                Box::new(move |cc| {
                    cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);

                    // The outer loop re-invokes `eframe::run_native` (and thus this closure)
                    // every time the window is restored from the tray, so GPU info must only
                    // be logged/tracked once per process to avoid duplicate logs and events.
                    if let Some(gl) = &cc.gl
                        && !GPU_INFO_REPORTED.swap(true, Ordering::Relaxed)
                    {
                        use eframe::glow::HasContext;
                        // SAFETY: Querying the renderer parameter string from a valid active glow OpenGL context is safe.
                        let renderer = unsafe { gl.get_parameter_string(eframe::glow::RENDERER) };
                        // SAFETY: Querying the vendor parameter string from a valid active glow OpenGL context is safe.
                        let vendor = unsafe { gl.get_parameter_string(eframe::glow::VENDOR) };
                        // SAFETY: Querying the version parameter string from a valid active glow OpenGL context is safe.
                        let version = unsafe { gl.get_parameter_string(eframe::glow::VERSION) };

                        log::info!(target: "Tracking", "GPU Renderer: {renderer}");
                        log::info!(target: "Tracking", "GPU Vendor: {vendor}");
                        log::info!(target: "Tracking", "OpenGL Version: {version}");

                        let displays = display_info::DisplayInfo::all().unwrap_or_default();
                        let max_hz = displays.iter().map(|d| d.frequency).fold(0.0, f32::max);

                        next_tablet_driver::app::telemetry::capture_event_with_set(
                            "gpu_initialized",
                            None,
                            Some(serde_json::json!({
                                "gpu_renderer": renderer,
                                "gpu_vendor": vendor,
                                "opengl_version": version,
                                "monitors_count": displays.len(),
                                "max_refresh_rate_hz": max_hz
                            })),
                        );
                    }

                    Ok(Box::new(TabletMapperApp::new(
                        &cc.egui_ctx,
                        ctx_shared,
                        ctx_config,
                        &ctx_corrections,
                        ctx_tablet_rx,
                        ctx_update_rx,
                        ctx_update_tx,
                        ctx_save_tx,
                    )))
                }),
            );

            if let Err(e) = result {
                log::error!(target: "App", "eframe error: {e}");
                return Err(e);
            }

            if !shared.lifecycle.is_visible.load(Ordering::Acquire) {
                log::info!(target: "App", "eframe exited, entering tray idle mode.");
            }
        } else {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
