//! # `NextTabletDriver` Entry Point
//!
//! This is the main executable for the `NextTabletDriver` application.
//! It initializes logging, checks for single-instance enforcement,
//! configures the window properties, and launches the `eframe` (egui) graphical interface.

#![windows_subsystem = "windows"]

use eframe::egui;
use next_tablet_driver::app::TabletMapperApp;
use next_tablet_driver::logger;
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

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
/// 1. Verifies no other instance is running.
/// 2. Initializes the application logger.
/// 3. Configures the GUI window options (icon, dimensions, title).
/// 4. Enters the `eframe::run_native` GUI event loop.
fn main() -> eframe::Result {
    logger::init();

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
                let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
                if ret != 0 {
                    log::error!(target: "Startup", "Another instance of NextTabletDriver is already running (PID locked).");
                    std::process::exit(1);
                }

                let _ = write!(f, "{}", std::process::id());
                Some(f) // Keeps flock alive for the process lifetime
            }
            Err(e) => {
                log::warn!(target: "Startup", "Could not create lock file at {:?}: {}", lock_path, e);
                None
            }
        }
    };

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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon_data)
            .with_inner_size([1000.0, 850.0])
            // .with_decorations(false)
            // .with_transparent(true)
            .with_title(format!("NextTabletDriver v{}", next_tablet_driver::VERSION)),
        ..Default::default()
    };

    eframe::run_native(
        &format!("NextTabletDriver v{}", next_tablet_driver::VERSION),
        options,
        Box::new(|cc| Ok(Box::new(TabletMapperApp::new(cc.egui_ctx.clone())))),
    )
}
