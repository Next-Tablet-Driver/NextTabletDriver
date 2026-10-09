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

    settings_dir_in(
        ProjectDirs::from("com", "NextTabletDriver", "NextTabletReader")
            .map(|dirs| dirs.config_dir().to_path_buf()),
    )
}

/// `<config_dir>/Settings`, created on demand; a relative `Settings` when the platform has no
/// per-user configuration directory.
fn settings_dir_in(config_dir: Option<PathBuf>) -> PathBuf {
    config_dir.map_or_else(
        || PathBuf::from("Settings"),
        |dir| {
            let settings = dir.join("Settings");
            if !settings.exists() {
                let _ = fs::create_dir_all(&settings);
            }
            settings
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ntd_paths_{name}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn without_a_platform_directory_the_settings_live_next_to_the_app() {
        assert_eq!(settings_dir_in(None), PathBuf::from("Settings"));
    }

    #[test]
    fn the_settings_folder_is_created_inside_the_config_directory() {
        let config = temp("settings_in");
        let settings = settings_dir_in(Some(config.clone()));
        assert_eq!(settings, config.join("Settings"));
        assert!(settings.is_dir());
        // A second call finds it again.
        assert_eq!(settings_dir_in(Some(config.clone())), settings);
        let _ = fs::remove_dir_all(config);
    }

    #[test]
    fn the_plugins_folder_is_created_on_demand() {
        let dir = temp("plugins");
        set_test_settings_dir(dir.clone());
        let plugins = get_plugins_dir();
        assert_eq!(plugins, dir.join("plugins"));
        assert!(plugins.is_dir());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn migrating_without_a_readable_settings_folder_does_nothing() {
        let dir = temp("unreadable");
        let file = dir.join("not_a_folder");
        fs::write(&file, "x").unwrap();
        set_test_settings_dir(file);
        migrate_profiles_to_subdir();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_legacy_profile_that_cannot_be_moved_stays_where_it_is() {
        log::set_max_level(log::LevelFilter::Trace);
        let dir = temp("blocked_migration");
        set_test_settings_dir(dir.clone());
        fs::write(dir.join("legacy.json"), "{}").unwrap();
        // `profiles` is a file, so nothing can be moved into it.
        fs::write(dir.join("profiles"), "not a folder").unwrap();
        migrate_profiles_to_subdir();
        assert!(dir.join("legacy.json").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_legacy_profile_never_replaces_a_profile_with_the_same_name() {
        let dir = temp("same_name");
        set_test_settings_dir(dir.clone());
        fs::create_dir_all(dir.join("profiles")).unwrap();
        fs::write(dir.join("legacy.json"), "old").unwrap();
        fs::write(dir.join("profiles").join("legacy.json"), "new").unwrap();
        migrate_profiles_to_subdir();
        assert_eq!(fs::read_to_string(dir.join("legacy.json")).unwrap(), "old");
        assert_eq!(
            fs::read_to_string(dir.join("profiles").join("legacy.json")).unwrap(),
            "new"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_legacy_profile_is_moved_into_the_profiles_folder() {
        log::set_max_level(log::LevelFilter::Trace);
        let dir = temp("moved");
        set_test_settings_dir(dir.clone());
        fs::write(dir.join("legacy.json"), "{}").unwrap();
        fs::write(dir.join("app_preferences.json"), "{}").unwrap();
        migrate_profiles_to_subdir();
        assert!(dir.join("profiles").join("legacy.json").exists());
        assert!(!dir.join("legacy.json").exists());
        // App-owned files stay in the root.
        assert!(dir.join("app_preferences.json").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
