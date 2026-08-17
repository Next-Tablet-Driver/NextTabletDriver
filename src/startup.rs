//! # Startup Management
//!
//! This module provides utilities to manage the application's lifecycle,
//! specifically handling the "run at startup" functionality.
//!
//! # Platform Specifics
//! - **Windows**: Creates/removes a `.lnk` shortcut in the user's Startup folder.
//! - **Linux**: Creates/removes a `.desktop` file in `~/.config/autostart/`.

use directories::UserDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

/// The name of the application used for shortcut/autostart naming.
const APP_NAME: &str = "NextTabletDriver";

// Windows Implementation .lnk shortcut in Startup folder
#[cfg(windows)]
mod platform {
    use super::{APP_NAME, PathBuf, UserDirs, env, fs};
    use std::process::Command;

    /// Returns the Windows Startup folder path for the current user.
    fn get_startup_folder() -> Option<PathBuf> {
        UserDirs::new().map(|dirs| {
            let mut path = dirs.home_dir().to_path_buf();
            path.push("AppData");
            path.push("Roaming");
            path.push("Microsoft");
            path.push("Windows");
            path.push("Start Menu");
            path.push("Programs");
            path.push("Startup");
            path
        })
    }

    /// Returns the full path where the application shortcut should be located.
    fn get_shortcut_path() -> Option<PathBuf> {
        get_startup_folder().map(|mut p| {
            p.push(format!("{APP_NAME}.lnk"));
            p
        })
    }

    /// Enables or disables the application's automatic launch at Windows startup.
    ///
    /// # Technical Details
    /// Windows `.lnk` files are a proprietary binary format. To avoid complex binary encoding,
    /// this function generates a temporary `VBScript` file, uses the `WScript.Shell` COM
    /// object to create the shortcut, executes the script via `wscript.exe`, then deletes
    /// the temporary script file.
    ///
    /// # Errors
    /// Returns an error if the shortcut path cannot be determined, environment variable access fails,
    /// or if the script execution fails.
    pub fn set_run_at_startup(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        let shortcut_path = get_shortcut_path().ok_or("Could not determine startup folder path")?;

        if enabled {
            let exe_path = env::current_exe()?;
            let exe_path_str = exe_path.to_str().ok_or("Invalid executable path")?;
            let shortcut_path_str = shortcut_path.to_str().ok_or("Invalid shortcut path")?;

            // VBScript + WScript.Shell COM: avoids encoding .lnk binary format directly
            let vbs_content = format!(
                r#"Set oWS = WScript.CreateObject("WScript.Shell")
            Set oLink = oWS.CreateShortcut("{}")
            oLink.TargetPath = "{}"
            oLink.WorkingDirectory = "{}"
            oLink.Save"#,
                shortcut_path_str.replace('\\', "\\\\"),
                exe_path_str.replace('\\', "\\\\"),
                exe_path
                    .parent()
                    .unwrap_or(&exe_path)
                    .to_str()
                    .unwrap_or("")
                    .replace('\\', "\\\\")
            );

            let temp_vbs = env::temp_dir().join("create_shortcut.vbs");
            fs::write(&temp_vbs, vbs_content)?;

            let status = Command::new("wscript").arg(&temp_vbs).status()?;

            let _ = fs::remove_file(temp_vbs);

            if !status.success() {
                return Err("Failed to create startup shortcut".into());
            }

            log::info!(target: "Startup", "Created startup shortcut: {}", shortcut_path.display());
        } else if shortcut_path.exists() {
            fs::remove_file(&shortcut_path)?;
            log::info!(target: "Startup", "Removed startup shortcut: {}", shortcut_path.display());
        }
        Ok(())
    }

    /// Checks if the application is currently configured to run at startup.
    #[must_use]
    pub fn is_run_at_startup_registered() -> bool {
        get_shortcut_path().is_some_and(|p| p.exists())
    }
}

// Linux Implementation .desktop file in ~/.config/autostart/
#[cfg(target_os = "linux")]
mod platform {
    use super::{APP_NAME, PathBuf, UserDirs, env, fs};

    /// Returns the path to the autostart directory: `~/.config/autostart/`.
    fn get_autostart_dir() -> std::path::PathBuf {
        let config_dir = env::var("XDG_CONFIG_HOME").map_or_else(
            |_| {
                UserDirs::new().map_or_else(
                    || PathBuf::from(".config"),
                    |dirs| dirs.home_dir().join(".config"),
                )
            },
            PathBuf::from,
        );
        config_dir.join("autostart")
    }

    /// Returns the full path to the `.desktop` autostart entry.
    fn get_desktop_path() -> PathBuf {
        let mut p = get_autostart_dir();
        p.push(format!("{APP_NAME}.desktop"));
        p
    }

    /// Enables or disables the application's automatic launch at session startup.
    ///
    /// Creates or removes a `.desktop` file following the XDG Autostart specification.
    /// # Errors
    /// Returns an error if the autostart directory cannot be determined, the executable
    /// path is invalid, or if file system operations fail.
    pub fn set_run_at_startup(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        let desktop_path = get_desktop_path();

        if enabled {
            if let Some(parent) = desktop_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let exe_path = env::current_exe()?;
            let exe_path_str = exe_path.to_str().ok_or("Invalid executable path")?;

            let desktop_content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name={APP_NAME}\n\
                 Comment=Tablet Driver for Osu! and Drawing\n\
                 Exec={exe_path_str}\n\
                 Terminal=false\n\
                 X-GNOME-Autostart-enabled=true\n\
                 StartupNotify=false\n"
            );

            fs::write(&desktop_path, desktop_content)?;
            log::info!(target: "Startup", "Created autostart entry: {}", desktop_path.display());
        } else if desktop_path.exists() {
            fs::remove_file(&desktop_path)?;
            log::info!(target: "Startup", "Removed autostart entry: {}", desktop_path.display());
        }
        Ok(())
    }

    /// Checks if the application is currently configured to run at startup.
    #[must_use]
    pub fn is_run_at_startup_registered() -> bool {
        get_desktop_path().exists()
    }
}

// Public re-exports unified cross-platform API
pub use platform::is_run_at_startup_registered;
pub use platform::set_run_at_startup;

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
    // SAFETY: mem_status is valid, dwLength is initialized, and GlobalMemoryStatusEx is a safe Win32 query.
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
    let username = std::env::var(if os == "windows" { "USERNAME" } else { "USER" })
        .unwrap_or_else(|_| "Unknown".to_string());
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

    log::info!(target: "Tracking", "Hostname: {hostname} | User: {username}");
    log::info!(target: "Tracking", "CPU Model: {cpu_identifier} | Cores: {num_processors}");
    if let Some(ram) = total_ram {
        log::info!(target: "Tracking", "Total Physical RAM: {:.2} GB", ram as f64 / 1_073_741_824.0);
    } else {
        log::info!(target: "Tracking", "Total Physical RAM: Unknown");
    }
}
