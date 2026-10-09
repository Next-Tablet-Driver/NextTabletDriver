use crate::core::config::models::ThemePreference;
use crate::i18n::Locale;
use serde::{Deserialize, Serialize};
use std::fs;

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

/// Saves application preferences to `app_preferences.json`.
pub fn save_app_preferences(prefs: &AppPreferences) {
    let path = super::get_settings_dir().join("app_preferences.json");
    write_json(&path, serde_json::to_string_pretty(prefs));
}

/// Writes serialized JSON through a temporary file, so a crash never leaves a partial file.
fn write_json(path: &std::path::Path, json: serde_json::Result<String>) {
    match json {
        Ok(json) => {
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = fs::write(&tmp, &json) {
                log::error!(target: "Config", "Failed to write temp app preferences: {e}");
            } else if let Err(e) = fs::rename(&tmp, path) {
                log::error!(target: "Config", "Failed to rename temp app preferences: {e}");
            } else {
                log::debug!(target: "Config", "Saved app preferences");
            }
        }
        Err(e) => {
            log::error!(target: "Config", "Failed to serialize app preferences: {e}");
        }
    }
}

/// Loads application preferences from `app_preferences.json`.
///
/// Returns `AppPreferences::default()` if the file doesn't exist or can't be parsed.
#[must_use]
pub fn load_app_preferences() -> AppPreferences {
    let path = super::get_settings_dir().join("app_preferences.json");
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(prefs) => {
                log::debug!(target: "Config", "Loaded app preferences");
                prefs
            }
            Err(e) => {
                log::error!(target: "Config", "Failed to parse app preferences: {e}");
                let prefs = AppPreferences::default();
                save_app_preferences(&prefs);
                prefs
            }
        },
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::error!(target: "Config", "Failed to read app preferences: {e}");
            }
            let prefs = AppPreferences::default();
            save_app_preferences(&prefs);
            prefs
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::settings::set_test_settings_dir;
    use std::path::PathBuf;

    /// A throw-away settings directory, removed on drop.
    struct TempSettings(PathBuf);

    impl TempSettings {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ntd_app_prefs_{name}_{nanos}"));
            fs::create_dir_all(&path).unwrap();
            log::set_max_level(log::LevelFilter::Trace);
            set_test_settings_dir(path.clone());
            Self(path)
        }

        fn file(&self) -> PathBuf {
            self.0.join("app_preferences.json")
        }
    }

    impl Drop for TempSettings {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
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
    fn save_then_load_round_trips() {
        let dir = TempSettings::new("roundtrip");
        let prefs = AppPreferences {
            theme: ThemePreference::Dark,
            language: Locale::French,
            telemetry_enabled: false,
            telemetry_id: "00000000-0000-4000-8000-000000000001".to_string(),
        };
        save_app_preferences(&prefs);
        assert!(dir.file().exists());
        // The atomic write leaves no temporary file behind.
        assert!(!dir.0.join("app_preferences.json.tmp").exists());

        let loaded = load_app_preferences();
        assert_eq!(loaded.theme, ThemePreference::Dark);
        assert_eq!(loaded.language, Locale::French);
        assert!(!loaded.telemetry_enabled);
        assert_eq!(loaded.telemetry_id, prefs.telemetry_id);
    }

    #[test]
    fn a_missing_file_yields_defaults_and_creates_the_file() {
        let dir = TempSettings::new("missing");
        assert!(!dir.file().exists());
        let loaded = load_app_preferences();
        assert!(loaded.telemetry_enabled);
        assert!(dir.file().exists());
        // The identifier that was created is the one that is reloaded next time.
        assert_eq!(load_app_preferences().telemetry_id, loaded.telemetry_id);
    }

    #[test]
    fn a_corrupt_file_is_replaced_by_valid_defaults() {
        let dir = TempSettings::new("corrupt");
        fs::write(dir.file(), "{ not json").unwrap();
        let loaded = load_app_preferences();
        assert!(loaded.telemetry_enabled);
        let rewritten: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.file()).unwrap()).unwrap();
        assert_eq!(rewritten["telemetry_id"], loaded.telemetry_id.as_str());
    }

    #[test]
    fn a_value_that_cannot_be_serialized_writes_nothing() {
        let dir = TempSettings::new("serialize_fail");
        let path = dir.0.join("prefs.json");
        write_json(&path, Err(serde_json::from_str::<u8>("x").unwrap_err()));
        assert!(!path.exists());
    }

    #[test]
    fn a_temporary_file_that_cannot_be_written_leaves_no_file() {
        let dir = TempSettings::new("write_fail");
        // The folder of the target does not exist, so the temporary file cannot be created.
        let path = dir.0.join("missing_folder").join("prefs.json");
        write_json(
            &path,
            serde_json::to_string_pretty(&AppPreferences::default()),
        );
        assert!(!path.exists());
    }

    #[test]
    fn a_target_that_cannot_be_replaced_is_left_alone() {
        let dir = TempSettings::new("rename_fail");
        // The target is a directory: the final rename cannot succeed.
        let path = dir.0.join("prefs.json");
        fs::create_dir_all(&path).unwrap();
        write_json(
            &path,
            serde_json::to_string_pretty(&AppPreferences::default()),
        );
        assert!(path.is_dir());
    }

    #[test]
    fn preferences_that_cannot_be_read_fall_back_to_the_defaults() {
        let dir = TempSettings::new("read_fail");
        fs::create_dir_all(dir.file()).unwrap();
        let loaded = load_app_preferences();
        assert!(loaded.telemetry_enabled);
        assert!(dir.file().is_dir());
    }
}
