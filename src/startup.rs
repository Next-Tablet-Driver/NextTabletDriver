//! # Startup Management
//!
//! This module adapts `ntd-platform` to the application (it finds the user's folders and logs a
//! failure of the timer request) and reports the hardware at startup. The OS calls themselves
//! (registry, `.desktop` file, timer resolution) live in `ntd-platform`.
use directories::UserDirs;
use ntd_platform::{PlatformError, UserPaths};
use std::env;
use std::path::PathBuf;

/// The folders of the current user that `ntd-platform` needs for autostart.
fn user_paths() -> UserPaths {
    let home = UserDirs::new().map_or_else(PathBuf::new, |dirs| dirs.home_dir().to_path_buf());
    let config = env::var_os("XDG_CONFIG_HOME").map_or_else(|| home.join(".config"), PathBuf::from);
    UserPaths { home, config }
}

/// Enables or disables the application's automatic launch at login.
///
/// # Errors
/// The error of `ntd-platform`: the registry or the file system refused, or the system has no
/// autostart.
pub fn set_run_at_startup(enabled: bool) -> Result<(), PlatformError> {
    ntd_platform::set_run_at_startup(enabled, &user_paths())
}

/// Checks if the application is currently configured to run at startup.
#[must_use]
pub fn is_run_at_startup_registered() -> bool {
    ntd_platform::is_run_at_startup_registered(&user_paths())
}

/// Asks for (or gives back) the finer timer resolution that Windows offers; a failure is only
/// logged, as the application works without it.
pub fn set_fast_timer(enable: bool) {
    if let Err(e) = ntd_platform::set_high_resolution_timer(enable) {
        log::warn!(target: "Timer", "Could not change the timer resolution: {e}");
    }
}

/// Gathers and logs detailed OS, CPU, Hostname, Username and physical memory (RAM) specifications.
pub fn log_system_hardware() {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let cpu_identifier =
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Unknown".to_string());
    let num_processors =
        std::env::var("NUMBER_OF_PROCESSORS").unwrap_or_else(|_| "Unknown".to_string());
    let system = ntd_platform::system_info();
    let hostname = system.hostname.unwrap_or_else(|| "Unknown".to_string());
    let total_ram = system.total_memory_bytes;

    log::info!(target: "Tracking", "OS: {os} | Architecture: {arch}");

    if let Some(distro) = system.distribution {
        log::info!(target: "Tracking", "Distribution: {distro}");
    }

    log::info!(target: "Tracking", "Hostname: {hostname}");
    log::info!(target: "Tracking", "CPU Model: {cpu_identifier} | Cores: {num_processors}");
    if let Some(ram) = total_ram {
        log::info!(target: "Tracking", "Total Physical RAM: {:.2} \
            GB", ram as f64 / 1_073_741_824.0);
    } else {
        log::info!(target: "Tracking", "Total Physical RAM: Unknown");
    }
}
