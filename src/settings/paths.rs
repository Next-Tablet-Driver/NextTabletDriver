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

/// Returns the directory where dynamic plugins are stored (`Settings/plugins/`).
#[must_use]
pub fn get_plugins_dir() -> PathBuf {
    let dir = get_settings_dir().join("plugins");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Returns `true` for the JSON files the app itself keeps in the root of `Settings/`.
///
/// User profiles live in their own `Settings/profiles/` directory, so listing profiles
/// never needs to filter by name; this is only used to tell a legacy profile apart from
/// an app-owned file when migrating the old flat layout.
fn is_system_file_stem(stem: &str) -> bool {
    matches!(
        stem,
        "last_session" | "session_meta" | "app_preferences" | "cache_releases"
    ) || stem.starts_with("crash_report")
}

/// Migrates any legacy profile JSON files from the root `Settings/` directory
/// into the `Settings/profiles/` subdirectory.
///
/// Files owned by the app itself (see [`is_system_file_stem`]) are left in place.
pub fn migrate_profiles_to_subdir() {
    let root = get_settings_dir();
    let profiles_dir = get_profiles_dir();

    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("json")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && !is_system_file_stem(stem)
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
