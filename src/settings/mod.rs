pub mod app_preferences;
pub mod otd_import;
#[cfg(feature = "gui")]
pub mod themes;

mod logging;
mod paths;
mod profile;
mod session;

pub use logging::log_mapping_config;
pub use paths::{get_profiles_dir, get_settings_dir, migrate_profiles_to_subdir};
pub use profile::{
    list_profiles, load_last_session, load_settings_from_file, sanitize_profile_name,
    save_last_session, save_settings, save_to_path,
};
pub use session::{SessionMeta, load_session_meta, save_session_meta};

#[cfg(test)]
pub use paths::set_test_settings_dir;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
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
