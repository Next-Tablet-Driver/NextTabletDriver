//! Settings/profile directory resolution and the legacy-to-subdir migration.

use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
thread_local! {
    static TEST_SETTINGS_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub fn set_test_settings_dir(path: PathBuf) {
    TEST_SETTINGS_DIR.with(|dir| {
        *dir.borrow_mut() = Some(path);
    });
}

#[must_use]
pub fn get_settings_dir() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(path) = TEST_SETTINGS_DIR.with(|dir| dir.borrow().clone()) {
            return path;
        }
    }

    ProjectDirs::from("com", "NextTabletDriver", "NextTabletReader").map_or_else(
        || PathBuf::from("Settings"),
        |proj_dirs| {
            let config_dir = proj_dirs.config_dir().join("Settings");
            if !config_dir.exists() {
                let _ = fs::create_dir_all(&config_dir);
            }
            config_dir
        },
    )
}

/// Returns the directory where user profile presets are stored (`Settings/profiles/`).
#[must_use]
pub fn get_profiles_dir() -> PathBuf {
    let dir = get_settings_dir().join("profiles");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Migrates any legacy profile JSON files from the root `Settings/` directory
/// into the `Settings/profiles/` subdirectory.
///
/// Only files that are not system files (e.g. `last_session`, `session_meta`,
/// `app_preferences`, `cache_releases`) are moved.
pub fn migrate_profiles_to_subdir() {
    let root = get_settings_dir();
    let profiles_dir = get_profiles_dir();
    let system_files = [
        "last_session",
        "session_meta",
        "app_preferences",
        "cache_releases",
    ];

    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("json")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && !system_files.contains(&stem)
        {
            let dest = profiles_dir.join(entry.file_name());
            if !dest.exists() {
                if let Err(e) = fs::rename(&path, &dest) {
                    log::warn!(target: "Config", "Failed to migrate profile '{stem}': {e}");
                } else {
                    log::info!(target: "Config", "Migrated profile '{stem}' to profiles/");
                }
            }
        }
    }
}
