//! # `ntd-settings`
//!
//! What the application keeps on disk: the profiles, the last session, the preferences and the
//! themes. Every folder comes from the [`Paths`] that the entry point of the program builds once
//! and passes down: this crate never looks for them (no global `get_*_dir()`).
//!
//! Every file is a JSON object with a `version` and is written atomically; a file that is not
//! valid is moved aside as `<name>.corrupt.bak` and reported, never overwritten (see [`store`]).

/// Preferences stored outside the profiles, such as the theme and the language.
pub mod app_preferences;
/// Import of `OpenTabletDriver` profiles.
pub mod otd_import;
pub mod store;
pub mod themes;

mod logging;
mod paths;
mod profile;
mod session;

pub use logging::log_mapping_config;
pub use paths::{Paths, migrate_profiles_to_subdir};
pub use profile::{
    LastSession, ProfileError, list_profiles, load_last_session, load_last_session_outcome,
    load_settings_from_file, sanitize_profile_name, save_last_session, save_settings, save_to_path,
};
pub use session::{SessionMeta, load_session_meta, save_session_meta};

#[cfg(test)]
mod tests {
    use super::*;
    use ntd_core::config::models::MappingConfig;
    use std::fs;
    use std::path::PathBuf;

    struct TempDirGuard {
        path: PathBuf,
        paths: Paths,
    }

    impl TempDirGuard {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::ZERO)
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ntd_tests_{name}_{nanos}"));
            fs::create_dir_all(&path).unwrap();
            let paths = Paths::new(path.clone());
            Self { path, paths }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_save_and_load_session_meta() {
        let guard = TempDirGuard::new("session_meta");

        let meta = SessionMeta {
            profile_name: "Default Profile".to_string(),
            profile_path: Some(PathBuf::from("C:\\some\\path.json")),
        };

        save_session_meta(&guard.paths, &meta);

        let loaded = load_session_meta(&guard.paths);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.profile_name, "Default Profile");
        assert_eq!(
            loaded.profile_path,
            Some(PathBuf::from("C:\\some\\path.json"))
        );
    }

    #[test]
    fn test_save_to_path_and_load_from_file() {
        let guard = TempDirGuard::new("save_load_path");

        let config = MappingConfig::default();
        let path = guard.paths.settings_dir().join("test_config.json");

        let res = save_to_path(&path, &config);
        assert!(res.is_ok());
        assert!(path.exists());

        let loaded_res = load_settings_from_file(&path);
        assert!(loaded_res.is_ok());
        let (loaded_config, corrections) = loaded_res.unwrap();
        assert!(corrections.is_empty());
        assert_eq!(loaded_config.tip_threshold, config.tip_threshold);
    }

    #[test]
    fn test_save_settings_and_list_profiles() {
        let guard = TempDirGuard::new("save_list_profiles");

        let config = MappingConfig::default();
        let save_res = save_settings(&guard.paths, "Game Profile", &config);
        assert!(save_res.is_ok());

        let profiles = list_profiles(&guard.paths);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].0, "Game Profile");
        assert!(profiles[0].1.exists());
    }

    #[test]
    fn test_migration_moves_only_legacy_profiles() {
        let guard = TempDirGuard::new("migrate_profiles");
        let root = guard.paths.settings_dir().to_path_buf();

        for name in [
            "Old Profile.json",
            "last_session.json",
            "session_meta.json",
            "app_preferences.json",
            "cache_releases.json",
            "crash_report.json",
        ] {
            fs::write(root.join(name), "{}").unwrap();
        }

        migrate_profiles_to_subdir(&guard.paths);

        let names: Vec<String> = list_profiles(&guard.paths)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(names, vec!["Old Profile".to_string()]);
        for kept in [
            "last_session.json",
            "session_meta.json",
            "app_preferences.json",
            "cache_releases.json",
            "crash_report.json",
        ] {
            assert!(
                root.join(kept).exists(),
                "{kept} must stay in the settings root"
            );
        }
    }

    #[test]
    fn test_profile_named_like_a_system_file_is_still_listed() {
        let guard = TempDirGuard::new("reserved_name_profile");
        save_settings(&guard.paths, "last_session", &MappingConfig::default()).unwrap();

        let names: Vec<String> = list_profiles(&guard.paths)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(names, vec!["last_session".to_string()]);
    }

    #[test]
    fn test_corrupt_last_session_is_preserved_not_overwritten() {
        let guard = TempDirGuard::new("corrupt_last_session");
        let path = guard.paths.settings_dir().join("last_session.json");
        fs::write(&path, "{ not json").unwrap();

        assert!(load_last_session(&guard.paths).is_none());
        assert!(!path.exists());
        let backup = guard
            .paths
            .settings_dir()
            .join("last_session.json.corrupt.bak");
        assert_eq!(fs::read_to_string(backup).unwrap(), "{ not json");
    }

    #[test]
    fn test_last_session() {
        let guard = TempDirGuard::new("last_session");

        let config = MappingConfig::default();
        let save_res = save_last_session(&guard.paths, &config);
        assert!(save_res.is_ok());

        let loaded = load_last_session(&guard.paths);
        assert!(loaded.is_some());
        let (loaded_config, corrections) = loaded.unwrap();
        assert!(corrections.is_empty());
        assert_eq!(loaded_config.tip_threshold, config.tip_threshold);
    }

    #[test]
    fn two_settings_folders_never_share_a_session() {
        let a = TempDirGuard::new("isolated_a");
        let b = TempDirGuard::new("isolated_b");
        save_last_session(&a.paths, &MappingConfig::default()).unwrap();
        assert!(load_last_session(&a.paths).is_some());
        assert!(load_last_session(&b.paths).is_none());
    }
}
