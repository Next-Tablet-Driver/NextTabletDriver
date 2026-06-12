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

fn default_telemetry_enabled() -> bool {
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
    match serde_json::to_string_pretty(prefs) {
        Ok(json) => {
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = fs::write(&tmp, &json) {
                log::error!(target: "Config", "Failed to write temp app preferences: {e}");
            } else if let Err(e) = fs::rename(&tmp, &path) {
                log::error!(target: "Config", "Failed to rename temp app preferences: {e}");
            } else {
                log::info!(target: "Config", "Saved app preferences");
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
                log::info!(target: "Config", "Loaded app preferences");
                prefs
            }
            Err(e) => {
                log::error!(target: "Config", "Failed to parse app preferences: {e}");
                AppPreferences::default()
            }
        },
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::error!(target: "Config", "Failed to read app preferences: {e}");
            }
            AppPreferences::default()
        }
    }
}

/// Validates that the selected theme still exists on disk, reverting to System if not.
pub fn validate_theme(prefs: &mut AppPreferences) {
    if let ThemePreference::Custom(name) = &prefs.theme {
        let available = crate::settings::themes::list_custom_themes();
        if !available.contains(name) {
            log::warn!(target: "Config", "Custom theme '{name}' not found, reverting to System");
            prefs.theme = ThemePreference::System;
        }
    }
}
