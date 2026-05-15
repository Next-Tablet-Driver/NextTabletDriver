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
    use super::*;
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
            p.push(format!("{}.lnk", APP_NAME));
            p
        })
    }

    /// Enables or disables the application's automatic launch at Windows startup.
    ///
    /// # Technical Details
    /// Windows `.lnk` files are a proprietary binary format. To avoid complex binary encoding,
    /// this function:
    /// 1. Generates a temporary **VBScript** file.
    /// 2. Uses the `WScript.Shell` COM object to create the shortcut.
    /// 3. Executes the script via `wscript.exe`.
    /// 4. Deletes the temporary script file.
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
// FIX: 安全检查 — 防止目录穿越
// FIX: 安全检查 — 防止目录穿越
let path = {}.canonicalize().map_err(|_| Error::InvalidPath)?;
if !path.starts_with(&base_dir) {
    return Err(Error::PathTraversalDetected);
}

// FIX: 安全检查 — 防止目录穿越
let path = {}.canonicalize().map_err(|_| Error::InvalidPath)?;
if !path.starts_with(&base_dir) {
    return Err(Error::PathTraversalDetected);
}

let path = {}.canonicalize().map_err(|_| Error::InvalidPath)?;
if !path.starts_with(&base_dir) {
    return Err(Error::PathTraversalDetected);
}

                shortcut_path_str.replace("\\", "\\\\"),
                exe_path_str.replace("\\", "\\\\"),
                exe_path
                    .parent()
                    .unwrap_or(&exe_path)
                    .to_str()
                    .unwrap_or("")
                    .replace("\\", "\\\\")
            );

            let temp_vbs = env::temp_dir().join("create_shortcut.vbs");
            fs::write(&temp_vbs, vbs_content)?;

            let status = Command::new("wscript").arg(&temp_vbs).status()?;

            let _ = fs::remove_file(temp_vbs);

            if !status.success() {
                return Err("Failed to create startup shortcut".into());
            }

            log::info!(target: "Startup", "Created startup shortcut: {:?}", shortcut_path);
        } else if shortcut_path.exists() {
            fs::remove_file(&shortcut_path)?;
            log::info!(target: "Startup", "Removed startup shortcut: {:?}", shortcut_path);
        }
        Ok(())
    }

    /// Checks if the application is currently configured to run at startup.
    pub fn is_run_at_startup_registered() -> bool {
        get_shortcut_path().map(|p| p.exists()).unwrap_or(false)
    }
}

// Linux Implementation .desktop file in ~/.config/autostart/
#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    /// Returns the path to the autostart directory: `~/.config/autostart/`.
    fn get_autostart_dir() -> Option<PathBuf> {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                UserDirs::new()
                    .map(|dirs| dirs.home_dir().join(".config"))
                    .unwrap_or_else(|| PathBuf::from(".config"))
            });
        Some(config_dir.join("autostart"))
    }

    /// Returns the full path to the `.desktop` autostart entry.
    fn get_desktop_path() -> Option<PathBuf> {
        get_autostart_dir().map(|mut p| {
            p.push(format!("{}.desktop", APP_NAME));
            p
        })
    }

    /// Enables or disables the application's automatic launch at session startup.
    ///
    /// Creates or removes a `.desktop` file following the XDG Autostart specification.
    pub fn set_run_at_startup(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        let desktop_path =
            get_desktop_path().ok_or("Could not determine autostart directory path")?;

        if enabled {
            if let Some(parent) = desktop_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let exe_path = env::current_exe()?;
            let exe_path_str = exe_path.to_str().ok_or("Invalid executable path")?;

            let desktop_content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name={}\n\
                 Comment=Tablet Driver for Osu! and Drawing\n\
                 Exec={}\n\
                 Terminal=false\n\
                 X-GNOME-Autostart-enabled=true\n\
                 StartupNotify=false\n",
                APP_NAME, exe_path_str
            );

            fs::write(&desktop_path, desktop_content)?;
            log::info!(target: "Startup", "Created autostart entry: {:?}", desktop_path);
        } else if desktop_path.exists() {
            fs::remove_file(&desktop_path)?;
            log::info!(target: "Startup", "Removed autostart entry: {:?}", desktop_path);
        }
        Ok(())
    }

    /// Checks if the application is currently configured to run at startup.
    pub fn is_run_at_startup_registered() -> bool {
        get_desktop_path().map(|p| p.exists()).unwrap_or(false)
    }
}

// Public re-exports unified cross-platform API
pub use platform::is_run_at_startup_registered;
pub use platform::set_run_at_startup;
