//! Linux autostart: a `.desktop` file in `~/.config/autostart/` (XDG Autostart).

use crate::{APP_NAME, PlatformError, UserPaths};
use std::path::PathBuf;
use std::{env, fs};

/// The full path of the `.desktop` autostart entry.
fn desktop_path(paths: &UserPaths) -> PathBuf {
    paths
        .config
        .join("autostart")
        .join(format!("{APP_NAME}.desktop"))
}

/// The content of the entry: it runs `exe` with `--autostart`.
fn desktop_entry(exe: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={APP_NAME}\n\
         Comment=Tablet Driver for Osu! and Drawing\n\
         Exec={exe} --autostart\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n\
         StartupNotify=false\n"
    )
}

/// Creates or removes the `.desktop` file.
pub(crate) fn set_run_at_startup(enabled: bool, paths: &UserPaths) -> Result<(), PlatformError> {
    let path = desktop_path(paths);
    let os_error = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", path.display()));

    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(os_error)?;
        }
        let exe = env::current_exe().map_err(os_error)?;
        let exe = exe
            .to_str()
            .ok_or_else(|| PlatformError::Os("Invalid executable path".to_string()))?;
        fs::write(&path, desktop_entry(exe)).map_err(os_error)?;
        log::info!(target: "Startup", "Created autostart entry: {}", path.display());
    } else if path.exists() {
        fs::remove_file(&path).map_err(os_error)?;
        log::info!(target: "Startup", "Removed autostart entry: {}", path.display());
    }
    Ok(())
}

/// Whether the `.desktop` file exists.
pub(crate) fn is_run_at_startup_registered(paths: &UserPaths) -> bool {
    desktop_path(paths).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(name: &str) -> UserPaths {
        let base = env::temp_dir().join(format!("ntd_platform_{name}_{}", std::process::id()));
        UserPaths {
            home: base.clone(),
            config: base.join(".config"),
        }
    }

    #[test]
    fn the_entry_runs_the_executable_with_the_autostart_flag() {
        let entry = desktop_entry("/opt/ntd/NextTabletDriver");
        assert!(entry.contains("Exec=/opt/ntd/NextTabletDriver --autostart\n"));
        assert!(entry.starts_with("[Desktop Entry]\n"));
    }

    #[test]
    fn enabling_then_disabling_creates_and_removes_the_entry() {
        let paths = paths("autostart");
        assert!(!is_run_at_startup_registered(&paths));

        set_run_at_startup(true, &paths).unwrap();
        assert!(is_run_at_startup_registered(&paths));
        assert!(
            desktop_path(&paths)
                .parent()
                .is_some_and(|dir| dir.ends_with("autostart"))
        );

        set_run_at_startup(false, &paths).unwrap();
        assert!(!is_run_at_startup_registered(&paths));
        // Removing what is not registered is not an error.
        set_run_at_startup(false, &paths).unwrap();
        let _ = fs::remove_dir_all(&paths.home);
    }
}
