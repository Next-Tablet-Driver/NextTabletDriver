use crate::paths::Paths;
use crate::store::{self, Loaded};
use ntd_core::config::models::ThemePreference;
use ntd_core::locale::Locale;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application-level preferences that are NOT tied to a specific tablet mapping profile.
///
/// Persisted to `app_preferences.json` in the settings directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppPreferences {
    /// The user's preferred UI theme.
    #[serde(default)]
    pub theme: ThemePreference,
    /// The user's preferred UI language.
    #[serde(default)]
    pub language: Locale,
    /// Whether anonymous telemetry is enabled.
    #[serde(default = "default_telemetry_enabled")]
    pub telemetry_enabled: bool,
    /// Anonymous distinct ID for telemetry.
    #[serde(default = "default_telemetry_id")]
    pub telemetry_id: String,
}

const fn default_telemetry_enabled() -> bool {
    true
}

fn default_telemetry_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            language: Locale::default(),
            telemetry_enabled: default_telemetry_enabled(),
            telemetry_id: default_telemetry_id(),
        }
    }
}

fn prefs_path(paths: &Paths) -> PathBuf {
    paths.settings_dir().join("app_preferences.json")
}

/// Saves application preferences to `app_preferences.json`.
pub fn save_app_preferences(paths: &Paths, prefs: &AppPreferences) {
    match store::write_json(&prefs_path(paths), prefs) {
        Ok(()) => log::debug!(target: "Config", "Saved app preferences"),
        Err(e) => log::error!(target: "Config", "Failed to save app preferences: {e}"),
    }
}

/// Loads application preferences from `app_preferences.json`.
///
/// Returns `AppPreferences::default()` if the file doesn't exist or can't be parsed. A file that
/// is not valid is moved aside as `app_preferences.json.corrupt.bak` before the defaults are
/// written, and a file written by a newer version is left alone (the defaults are used for this
/// run only).
#[must_use]
pub fn load_app_preferences(paths: &Paths) -> AppPreferences {
    match store::read_json(&prefs_path(paths)) {
        Loaded::Ok { value, .. } => {
            log::debug!(target: "Config", "Loaded app preferences");
            value
        }
        Loaded::TooNew { found } => {
            log::error!(target: "Config", "The app preferences were written by a newer version \
                ({found}) and are left as they are");
            AppPreferences::default()
        }
        Loaded::Missing | Loaded::Corrupt { .. } => {
            let prefs = AppPreferences::default();
            save_app_preferences(paths, &prefs);
            prefs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A throw-away settings directory, removed on drop.
    struct TempSettings {
        root: PathBuf,
        paths: Paths,
    }

    impl TempSettings {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("ntd_app_prefs_{name}_{nanos}"));
            fs::create_dir_all(&root).unwrap();
            let paths = Paths::new(root.clone());
            Self { root, paths }
        }

        fn file(&self) -> PathBuf {
            self.root.join("app_preferences.json")
        }
    }

    impl Drop for TempSettings {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn defaults_enable_telemetry_with_a_fresh_uuid() {
        let a = AppPreferences::default();
        let b = AppPreferences::default();
        assert!(a.telemetry_enabled);
        assert_eq!(a.theme, ThemePreference::System);
        assert_eq!(a.language, Locale::English);
        assert!(uuid::Uuid::parse_str(&a.telemetry_id).is_ok());
        assert_ne!(a.telemetry_id, b.telemetry_id);
    }

    #[test]
    fn missing_fields_fall_back_to_their_defaults() {
        let prefs: AppPreferences = serde_json::from_str("{}").unwrap();
        assert!(prefs.telemetry_enabled);
        assert_eq!(prefs.theme, ThemePreference::System);
        assert!(uuid::Uuid::parse_str(&prefs.telemetry_id).is_ok());

        let prefs: AppPreferences =
            serde_json::from_str(r#"{ "language": "French", "telemetry_enabled": false }"#)
                .unwrap();
        assert_eq!(prefs.language, Locale::French);
        assert!(!prefs.telemetry_enabled);
    }

    #[test]
    fn save_then_load_round_trips_with_its_version() {
        let dir = TempSettings::new("roundtrip");
        let prefs = AppPreferences {
            theme: ThemePreference::Dark,
            language: Locale::French,
            telemetry_enabled: false,
            telemetry_id: "00000000-0000-4000-8000-000000000001".to_string(),
        };
        save_app_preferences(&dir.paths, &prefs);
        assert!(dir.file().exists());
        assert!(
            fs::read_to_string(dir.file())
                .unwrap()
                .contains("\"version\": 1")
        );
        // The atomic write leaves no temporary file behind.
        assert!(!dir.root.join("app_preferences.json.tmp").exists());

        let loaded = load_app_preferences(&dir.paths);
        assert_eq!(loaded.theme, ThemePreference::Dark);
        assert_eq!(loaded.language, Locale::French);
        assert!(!loaded.telemetry_enabled);
        assert_eq!(loaded.telemetry_id, prefs.telemetry_id);
    }

    #[test]
    fn preferences_of_v2_without_a_version_still_load() {
        let dir = TempSettings::new("legacy");
        fs::write(
            dir.file(),
            r#"{ "language": "French", "telemetry_enabled": false }"#,
        )
        .unwrap();
        let loaded = load_app_preferences(&dir.paths);
        assert_eq!(loaded.language, Locale::French);
        assert!(!loaded.telemetry_enabled);
    }

    #[test]
    fn a_missing_file_yields_defaults_and_creates_the_file() {
        let dir = TempSettings::new("missing");
        assert!(!dir.file().exists());
        let loaded = load_app_preferences(&dir.paths);
        assert!(loaded.telemetry_enabled);
        assert!(dir.file().exists());
        // The identifier that was created is the one that is reloaded next time.
        assert_eq!(
            load_app_preferences(&dir.paths).telemetry_id,
            loaded.telemetry_id
        );
    }

    #[test]
    fn a_corrupt_file_is_backed_up_then_replaced_by_valid_defaults() {
        let dir = TempSettings::new("corrupt");
        fs::write(dir.file(), "{ not json").unwrap();
        let loaded = load_app_preferences(&dir.paths);
        assert!(loaded.telemetry_enabled);
        let rewritten: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.file()).unwrap()).unwrap();
        assert_eq!(rewritten["telemetry_id"], loaded.telemetry_id.as_str());
        assert_eq!(
            fs::read_to_string(dir.root.join("app_preferences.json.corrupt.bak")).unwrap(),
            "{ not json"
        );
    }

    #[test]
    fn preferences_of_a_newer_version_are_not_overwritten() {
        let dir = TempSettings::new("newer");
        let text = r#"{ "version": 77, "telemetry_enabled": false }"#;
        fs::write(dir.file(), text).unwrap();
        let loaded = load_app_preferences(&dir.paths);
        assert!(loaded.telemetry_enabled);
        assert_eq!(fs::read_to_string(dir.file()).unwrap(), text);
    }

    #[test]
    fn preferences_that_cannot_be_read_fall_back_to_the_defaults() {
        let dir = TempSettings::new("read_fail");
        fs::create_dir_all(dir.file()).unwrap();
        let loaded = load_app_preferences(&dir.paths);
        assert!(loaded.telemetry_enabled);
        assert!(dir.file().is_dir());
    }
}
