//! Profile file I/O: save/load presets and the last-session snapshot.

use super::paths::{get_profiles_dir, get_settings_dir};
use crate::core::config::models::MappingConfig;
use std::fs;
use std::path::{Path, PathBuf};

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
    let dir = get_profiles_dir();
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
                let corrections = config.validate_and_repair();

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

    let corrections = config.validate_and_repair();

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
    let dir = get_profiles_dir();
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
            && stem != "cache_releases"
        {
            profiles.push((stem.to_string(), path));
        }
    }

    profiles.sort_by_key(|a| a.0.to_lowercase());
    profiles
}
