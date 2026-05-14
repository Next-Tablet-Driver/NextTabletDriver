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

#[must_use]
pub fn get_settings_dir() -> PathBuf {
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
pub fn save_to_path(path: &Path, config: &MappingConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| {
        log::error!(target: "Config", "Failed to serialize config for {path:?}: {e}");
        e.to_string()
    })?;

    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json).map_err(|e| {
        log::error!(target: "Config", "Failed to write temp file {tmp_path:?}: {e}");
        e.to_string()
    })?;

    fs::rename(&tmp_path, path).map_err(|e| {
        log::error!(target: "Config", "Failed to rename {tmp_path:?} -> {path:?}: {e}");
        // Clean up the orphaned temp file on rename failure
        let _ = fs::remove_file(&tmp_path);
        e.to_string()
    })?;

    Ok(())
}

/// Sanitizes a string for use as a filename, removing path separators and reserved characters.
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
    log::info!(target: "Config", "Saved preset '{}' (sanitized: '{}') to {:?}", name, sanitized_name, path);
    Ok(())
}

/// Persists the current session state to `last_session.json`.
///
/// Called asynchronously from a background saver thread — never from the UI thread.
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
        log::debug!(target: "Config", "No last session file found at {path:?}");
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
            Ok(mut config) => {
                let corrections = config.validate_and_repair();
                if !corrections.is_empty() {
                    log::warn!(target: "Config", "Last session config had {} field(s) repaired", corrections.len());
                }
                log::info!(target: "Config", "Loaded last session from {path:?}");
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
pub fn load_settings_from_file(path: &Path) -> Result<(MappingConfig, Vec<String>), String> {
    let content = fs::read_to_string(path).map_err(|e| {
        log::error!(target: "Config", "Failed to read settings file {path:?}: {e}");
        e.to_string()
    })?;
    let mut config: MappingConfig = serde_json::from_str(&content).map_err(|e| {
        log::error!(target: "Config", "Failed to parse settings JSON from {path:?}: {e}");
        e.to_string()
    })?;

    let corrections = config.validate_and_repair();
    if !corrections.is_empty() {
        log::warn!(target: "Config", "Config from {path:?} had {} field(s) repaired", corrections.len());
    }

    log::info!(target: "Config", "Loaded settings from {path:?}");
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
            log::error!(target: "Config", "Failed to list profiles in {dir:?}: {e}");
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
