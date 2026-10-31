//! # Startup Management
//!
//! This module adapts `ntd-platform` to the application (it finds the user's folders and logs a
//! failure of the timer request) and reports the hardware at startup. The OS calls themselves
//! (registry, `.desktop` file, timer resolution) live in `ntd-platform`.
#![allow(
    unsafe_code,
    reason = "the memory query of Windows through windows-sys"
)]

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

/// Queries the operating system for the total amount of physical memory (RAM) in bytes.
///
/// On Windows, queries `GlobalMemoryStatusEx`.
#[cfg(windows)]
#[must_use]
pub fn get_memory_info() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut mem_status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };
    // SAFETY: mem_status is valid, dwLength is initialized, and GlobalMemoryStatusEx is a safe
    // Win32 query.
    let success = unsafe { GlobalMemoryStatusEx(&raw mut mem_status) };
    if success != 0 {
        Some(mem_status.ullTotalPhys)
    } else {
        None
    }
}

/// Queries the operating system for the total amount of physical memory (RAM) in bytes.
///
/// On Linux, parses `/proc/meminfo`.
#[cfg(not(windows))]
#[must_use]
pub fn get_memory_info() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(kb) = parts.get(1).and_then(|s| s.parse::<u64>().ok()) {
                        return Some(kb * 1024); // KB to Bytes
                    }
                }
            }
        }
    }
    None
}

/// Reads `/etc/os-release` to extract the system's human-readable distribution name.
#[cfg(target_os = "linux")]
fn get_linux_distro() -> Option<String> {
    if let Ok(release) = std::fs::read_to_string("/etc/os-release") {
        for line in release.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Gathers and logs detailed OS, CPU, Hostname, Username and physical memory (RAM) specifications.
pub fn log_system_hardware() {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let cpu_identifier =
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Unknown".to_string());
    let num_processors =
        std::env::var("NUMBER_OF_PROCESSORS").unwrap_or_else(|_| "Unknown".to_string());
    let hostname = std::env::var(if os == "windows" {
        "COMPUTERNAME"
    } else {
        "HOSTNAME"
    })
    .unwrap_or_else(|_| {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/etc/hostname")
                .map_or_else(|_| "Unknown".to_string(), |s| s.trim().to_string())
        }
        #[cfg(not(target_os = "linux"))]
        {
            "Unknown".to_string()
        }
    });

    let total_ram = get_memory_info();

    log::info!(target: "Tracking", "OS: {os} | Architecture: {arch}");

    #[cfg(target_os = "linux")]
    if let Some(distro) = get_linux_distro() {
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
