pub mod themes;

use crate::core::config::models::MappingConfig;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Lightweight metadata about the active profile, persisted across restarts.
///
/// Stored in `session_meta.json` alongside `last_session.json`.
/// This allows the driver to silently restore the profile identity
/// (name + file path) on startup without re-prompting the user.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMeta {
    /// Display name of the active profile.
    pub profile_name: String,
    /// Absolute path to the profile file on disk, if any.
    pub profile_path: Option<PathBuf>,
}

/// Saves the active profile metadata to `session_meta.json`.
pub fn save_session_meta(meta: &SessionMeta) {
    let path = get_settings_dir().join("session_meta.json");
    if let Ok(json) = serde_json::to_string_pretty(meta) {
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &json).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }
}

/// Loads the active profile metadata from `session_meta.json`.
#[must_use]
pub fn load_session_meta() -> Option<SessionMeta> {
    let path = get_settings_dir().join("session_meta.json");
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

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

/// Atomically writes a `MappingConfig` to an arbitrary path on disk.
///
/// Uses a write-to-temp-then-rename strategy to prevent corruption
/// if the process crashes mid-write.
///
/// # Errors
/// Returns an error string if serialization fails, the temporary file cannot be written,
/// or the final rename operation fails.
pub fn save_to_path(path: &Path, config: &MappingConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| {
        log::error!(target: "Config", "Failed to serialize config for {}: {e}", path.display());
        e.to_string()
    })?;

    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json).map_err(|e| {
        log::error!(target: "Config", "Failed to write temp file {}: {e}", tmp_path.display());
        e.to_string()
    })?;

    fs::rename(&tmp_path, path).map_err(|e| {
        log::error!(target: "Config", "Failed to rename {}: {e}", tmp_path.display());
        // Clean up the orphaned temp file on rename failure
        let _ = fs::remove_file(&tmp_path);
        e.to_string()
    })?;

    Ok(())
}

/// Sanitizes a string for use as a filename, removing path separators and reserved characters.
#[must_use]
pub fn sanitize_profile_name(name: &str) -> String {
    let mut sanitized: String = name
        .chars()
        .filter(|&c| {
            // Filter out characters that are invalid in Windows filenames and path separators
            !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .collect();

    // Prevent directory traversal sequences and hidden files
    sanitized = sanitized
        .replace("..", "")
        .trim_start_matches('.')
        .to_string();

    // Fallback if the name becomes empty after sanitization
    if sanitized.is_empty() {
        "unnamed_profile".to_string()
    } else {
        sanitized
    }
}

/// Saves config as a named preset in the application's settings directory.
///
/// # Errors
/// Returns an error if the profile name cannot be sanitized or if `save_to_path` fails.
pub fn save_settings(name: &str, config: &MappingConfig) -> Result<(), String> {
    let dir = get_settings_dir();
    let sanitized_name = sanitize_profile_name(name);

    let filename = if sanitized_name.to_lowercase().ends_with(".json") {
        sanitized_name.clone()
    } else {
        format!("{sanitized_name}.json")
    };

    let path = dir.join(&filename);

    save_to_path(&path, config)?;
    log::info!(target: "Config", "Saved preset '{name}' (sanitized: '{sanitized_name}') to {}", path.display());
    Ok(())
}

/// Validates that the selected theme exists, reverting to System if it doesn't.
fn validate_theme(config: &mut MappingConfig, corrections: &mut Vec<String>) {
    if let crate::core::config::models::ThemePreference::Custom(name) = &config.theme {
        let available = crate::settings::themes::list_custom_themes();
        if !available.contains(name) {
            corrections.push(format!(
                "Custom theme '{name}' not found, reverting to System"
            ));
            config.theme = crate::core::config::models::ThemePreference::System;
        }
    }
}

/// Persists the current session state to `last_session.json`.
///
/// Called asynchronously from a background saver thread - never from the UI thread.
///
/// # Errors
/// Returns an error if the settings directory cannot be resolved or if `save_to_path` fails.
pub fn save_last_session(config: &MappingConfig) -> Result<(), String> {
    let path = get_settings_dir().join("last_session.json");
    save_to_path(&path, config)?;
    log::trace!(target: "Config", "Last session persistent state updated");
    Ok(())
}

/// Loads the last session config, running validation and repair on the result.
///
/// Returns `None` if no session file exists. Returns `Some((config, corrections))`
/// where `corrections` is a list of fields that were repaired (empty if all valid).
#[must_use]
pub fn load_last_session() -> Option<(MappingConfig, Vec<String>)> {
    let path = get_settings_dir().join("last_session.json");
    if !path.exists() {
        log::debug!(target: "Config", "No last session file found at {}", path.display());
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
            Ok(mut config) => {
                let mut corrections = config.validate_and_repair();
                validate_theme(&mut config, &mut corrections);

                if !corrections.is_empty() {
                    log::warn!(target: "Config", "Last session config had {} field(s) repaired", corrections.len());
                    // Automatically save the repaired config
                    let _ = save_last_session(&config);
                }
                log::info!(target: "Config", "Loaded last session from {}", path.display());
                Some((config, corrections))
            }
            Err(e) => {
                log::error!(target: "Config", "Failed to parse last session JSON: {e}");
                None
            }
        },
        Err(e) => {
            log::error!(target: "Config", "Failed to read last session file: {e}");
            None
        }
    }
}

/// Loads and validates a config from an arbitrary file path.
///
/// Returns the config and a list of corrections applied during validation.
///
/// # Errors
/// Returns an error if the file cannot be read or if the JSON content is invalid.
pub fn load_settings_from_file(path: &Path) -> Result<(MappingConfig, Vec<String>), String> {
    let content = fs::read_to_string(path).map_err(|e| {
        log::error!(target: "Config", "Failed to read settings file {}: {e}", path.display());
        e.to_string()
    })?;
    let mut config: MappingConfig = serde_json::from_str(&content).map_err(|e| {
        log::error!(target: "Config", "Failed to parse settings JSON from {}: {e}", path.display());
        e.to_string()
    })?;

    let mut corrections = config.validate_and_repair();
    validate_theme(&mut config, &mut corrections);

    if !corrections.is_empty() {
        log::warn!(target: "Config", "Config from {} had {} field(s) repaired", path.display(), corrections.len());
        // Automatically save the repaired config
        let _ = save_to_path(path, &config);
    }

    log::info!(target: "Config", "Loaded settings from {}", path.display());
    Ok((config, corrections))
}

/// Lists all saved profile files in the settings directory.
///
/// Returns `(display_name, absolute_path)` pairs, excluding `last_session.json`.
#[must_use]
pub fn list_profiles() -> Vec<(String, PathBuf)> {
    let dir = get_settings_dir();
    let mut profiles = Vec::new();

    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!(target: "Config", "Failed to list profiles in {}: {e}", dir.display());
            return profiles;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && stem != "last_session"
            && stem != "session_meta"
        {
            profiles.push((stem.to_string(), path));
        }
    }

    profiles.sort_by_key(|a| a.0.to_lowercase());
    profiles
}

/// Logs the current driver mapping configuration settings to the tracking log target.
pub fn log_mapping_config(config: &MappingConfig, prefix: &str) {
    log::info!(target: "Tracking", "=== CONFIGURATION LOG ({prefix}) ===");
    log::info!(target: "Tracking", "Mode: {:?}", config.mode);
    log::info!(
        target: "Tracking",
        "Active Area -> Width: {:.2}, Height: {:.2} | Offsets -> X: {:.2}, Y: {:.2} | Rotation: {:.1} deg",
        config.active_area.w,
        config.active_area.h,
        config.active_area.x,
        config.active_area.y,
        config.active_area.rotation
    );
    log::info!(
        target: "Tracking",
        "Target Area -> Width: {:.2}, Height: {:.2} | Offsets -> X: {:.2}, Y: {:.2}",
        config.target_area.w,
        config.target_area.h,
        config.target_area.x,
        config.target_area.y
    );
    log::info!(
        target: "Tracking",
        "Antichatter -> Enabled: {} | Latency: {:.1}ms | Strength: {:.2}",
        config.antichatter.enabled,
        config.antichatter.latency,
        config.antichatter.antichatter_strength
    );
    log::info!(
        target: "Tracking",
        "Stylus -> Tip Threshold: {} | Eraser Threshold: {} | Disable Pressure: {} | Disable Tilt: {}",
        config.tip_threshold,
        config.eraser_threshold,
        config.disable_pressure,
        config.disable_tilt
    );
    log::info!(
        target: "Tracking",
        "General -> UI Theme: {:?} | Lock Aspect Ratio: {} | Show Playfield: {} | UI Language: {:?}",
        config.theme,
        config.lock_aspect_ratio,
        config.show_osu_playfield,
        config.language
    );
    log::info!(target: "Tracking", "========================================");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use std::fs;

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
