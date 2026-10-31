//! Where the settings live, and the legacy-to-subdir migration.
//!
//! The folders are decided once, at the entry point of the program (the desktop app, the SDK),
//! and passed down as a [`Paths`]. Nothing in the workspace looks for them on its own, so a test
//! gives each case a folder of its own.

use std::fs;
use std::path::{Path, PathBuf};

/// The folders the application reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    settings: PathBuf,
}

impl Paths {
    /// The settings live in `settings_root`, which is created when it is missing.
    #[must_use]
    pub fn new(settings_root: PathBuf) -> Self {
        if !settings_root.exists() {
            let _ = fs::create_dir_all(&settings_root);
        }
        Self {
            settings: settings_root,
        }
    }

    /// The folder of profiles, sessions, preferences and the other files of the application.
    #[must_use]
    pub fn settings_dir(&self) -> &Path {
        &self.settings
    }

    /// The folder where the logs are written: the settings folder.
    #[must_use]
    pub fn logs_dir(&self) -> &Path {
        &self.settings
    }

    /// The folder of the user's profile presets (`profiles/`), created on demand.
    #[must_use]
    pub fn profiles_dir(&self) -> PathBuf {
        self.subdir("profiles")
    }

    /// The folder of the dynamic plugins (`plugins/`), created on demand.
    #[must_use]
    pub fn plugins_dir(&self) -> PathBuf {
        self.subdir("plugins")
    }

    /// The folder of the user's themes (`Themes/`), created on demand.
    #[must_use]
    pub fn themes_dir(&self) -> PathBuf {
        self.subdir("Themes")
    }

    fn subdir(&self, name: &str) -> PathBuf {
        let dir = self.settings.join(name);
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        dir
    }
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
pub fn migrate_profiles_to_subdir(paths: &Paths) {
    let root = paths.settings_dir();
    let profiles_dir = paths.profiles_dir();

    let Ok(entries) = fs::read_dir(root) else {
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
    fn the_settings_root_is_created_when_it_is_missing() {
        let base = temp("root");
        let root = base.join("Settings");
        let paths = Paths::new(root.clone());
        assert_eq!(paths.settings_dir(), root);
        assert!(root.is_dir());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn the_subfolders_are_created_on_demand_under_the_root() {
        let dir = temp("subdirs");
        let paths = Paths::new(dir.clone());
        for (folder, name) in [
            (paths.profiles_dir(), "profiles"),
            (paths.plugins_dir(), "plugins"),
            (paths.themes_dir(), "Themes"),
        ] {
            assert_eq!(folder, dir.join(name));
            assert!(folder.is_dir());
        }
        assert_eq!(paths.logs_dir(), dir);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn two_paths_on_two_roots_do_not_see_each_other() {
        let a = Paths::new(temp("a"));
        let b = Paths::new(temp("b"));
        fs::write(a.profiles_dir().join("only_a.json"), "{}").unwrap();
        assert!(!b.profiles_dir().join("only_a.json").exists());
        let _ = fs::remove_dir_all(a.settings_dir());
        let _ = fs::remove_dir_all(b.settings_dir());
    }

    #[test]
    fn migrating_without_a_readable_settings_folder_does_nothing() {
        let dir = temp("unreadable");
        let file = dir.join("not_a_folder");
        fs::write(&file, "x").unwrap();
        migrate_profiles_to_subdir(&Paths { settings: file });
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_legacy_profile_that_cannot_be_moved_stays_where_it_is() {
        let dir = temp("blocked_migration");
        let paths = Paths {
            settings: dir.clone(),
        };
        fs::write(dir.join("legacy.json"), "{}").unwrap();
        // `profiles` is a file, so nothing can be moved into it.
        fs::write(dir.join("profiles"), "not a folder").unwrap();
        migrate_profiles_to_subdir(&paths);
        assert!(dir.join("legacy.json").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_legacy_profile_never_replaces_a_profile_with_the_same_name() {
        let dir = temp("same_name");
        let paths = Paths::new(dir.clone());
        fs::create_dir_all(dir.join("profiles")).unwrap();
        fs::write(dir.join("legacy.json"), "old").unwrap();
        fs::write(dir.join("profiles").join("legacy.json"), "new").unwrap();
        migrate_profiles_to_subdir(&paths);
        assert_eq!(fs::read_to_string(dir.join("legacy.json")).unwrap(), "old");
        assert_eq!(
            fs::read_to_string(dir.join("profiles").join("legacy.json")).unwrap(),
            "new"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_legacy_profile_is_moved_into_the_profiles_folder() {
        let dir = temp("moved");
        let paths = Paths::new(dir.clone());
        fs::write(dir.join("legacy.json"), "{}").unwrap();
        fs::write(dir.join("app_preferences.json"), "{}").unwrap();
        migrate_profiles_to_subdir(&paths);
        assert!(dir.join("profiles").join("legacy.json").exists());
        assert!(!dir.join("legacy.json").exists());
        // App-owned files stay in the root.
        assert!(dir.join("app_preferences.json").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
