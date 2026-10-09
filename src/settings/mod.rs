pub mod app_preferences;
pub mod otd_import;
pub mod themes;

mod logging;
mod paths;
mod profile;
mod session;

pub use logging::log_mapping_config;
pub use paths::{get_plugins_dir, get_profiles_dir, get_settings_dir, migrate_profiles_to_subdir};
pub use profile::{
    list_profiles, load_last_session, load_settings_from_file, sanitize_profile_name,
    save_last_session, save_settings, save_to_path,
};
pub use session::{SessionMeta, load_session_meta, save_session_meta};

#[cfg(test)]
pub use paths::set_test_settings_dir;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::models::MappingConfig;
    use std::fs;
    use std::path::PathBuf;

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::ZERO)
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ntd_tests_{name}_{nanos}"));
            fs::create_dir_all(&path).unwrap();
            set_test_settings_dir(path.clone());
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_sanitize_profile_name() {
        assert_eq!(sanitize_profile_name("my_profile"), "my_profile");
        assert_eq!(sanitize_profile_name("my/profile\\name"), "myprofilename");
        assert_eq!(sanitize_profile_name("profile:test?*"), "profiletest");
        assert_eq!(sanitize_profile_name("../../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_profile_name("..\\invalid"), "invalid");
        assert_eq!(sanitize_profile_name(".hidden"), "hidden");
        assert_eq!(sanitize_profile_name("/\\:*?\"<>|"), "unnamed_profile");
    }

    #[test]
    fn test_save_and_load_session_meta() {
        let _guard = TempDirGuard::new("session_meta");

        let meta = SessionMeta {
            profile_name: "Default Profile".to_string(),
            profile_path: Some(PathBuf::from("C:\\some\\path.json")),
        };

        save_session_meta(&meta);

        let loaded = load_session_meta();
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
        let _guard = TempDirGuard::new("save_load_path");

        let config = MappingConfig::default();
        let path = get_settings_dir().join("test_config.json");

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
        let _guard = TempDirGuard::new("save_list_profiles");

        let config = MappingConfig::default();
        let save_res = save_settings("Game Profile", &config);
        assert!(save_res.is_ok());

        let profiles = list_profiles();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].0, "Game Profile");
        assert!(profiles[0].1.exists());
    }

    #[test]
    fn test_migration_moves_only_legacy_profiles() {
        let guard = TempDirGuard::new("migrate_profiles");
        let root = get_settings_dir();

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

        migrate_profiles_to_subdir();

        let names: Vec<String> = list_profiles().into_iter().map(|(n, _)| n).collect();
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
        drop(guard);
    }

    #[test]
    fn test_profile_named_like_a_system_file_is_still_listed() {
        let _guard = TempDirGuard::new("reserved_name_profile");
        save_settings("last_session", &MappingConfig::default()).unwrap();

        let names: Vec<String> = list_profiles().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["last_session".to_string()]);
    }

    #[test]
    fn test_corrupt_last_session_is_preserved_not_overwritten() {
        let _guard = TempDirGuard::new("corrupt_last_session");
        let path = get_settings_dir().join("last_session.json");
        fs::write(&path, "{ not json").unwrap();

        assert!(load_last_session().is_none());
        assert!(!path.exists());
        let backup = get_settings_dir().join("last_session.json.corrupt.bak");
        assert_eq!(fs::read_to_string(backup).unwrap(), "{ not json");
    }

    #[test]
    fn test_last_session() {
        let _guard = TempDirGuard::new("last_session");

        let config = MappingConfig::default();
        let save_res = save_last_session(&config);
        assert!(save_res.is_ok());

        let loaded = load_last_session();
        assert!(loaded.is_some());
        let (loaded_config, corrections) = loaded.unwrap();
        assert!(corrections.is_empty());
        assert_eq!(loaded_config.tip_threshold, config.tip_threshold);
    }
}
