//! Profile file I/O: save/load presets and the last-session snapshot.

use super::paths::{get_profiles_dir, get_settings_dir};
use crate::core::config::models::MappingConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// Moves an unreadable/invalid settings file out of the way (as `<name>.corrupt.bak`) so
/// the next save does not silently overwrite what the user may still want to recover.
fn quarantine_corrupt_file(path: &Path) {
    let mut backup = path.as_os_str().to_owned();
    backup.push(".corrupt.bak");
    let backup = PathBuf::from(backup);
    match fs::rename(path, &backup) {
        Ok(()) => log::warn!(
            target: "Config",
            "Invalid settings file preserved as {}",
            backup.display()
        ),
        Err(e) => log::error!(
            target: "Config",
            "Failed to back up invalid settings file {}: {e}",
            path.display()
        ),
    }
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
    save_json_to_path(path, config)
}

fn save_json_to_path<T: serde::Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }

    let json = serde_json::to_string_pretty(value).map_err(|e| {
        log::error!(target: "Config", "Failed to serialize config for {}: {e}", path.display());
        e.to_string()
    })?;

    let tmp_path = path.with_extension("json.tmp");
    // fsync before the rename so a power loss cannot leave an empty/partial file in place.
    let write_result = fs::File::create(&tmp_path).and_then(|mut file| {
        use std::io::Write;
        file.write_all(json.as_bytes())?;
        file.sync_all()
    });
    write_result.map_err(|e| {
        log::error!(target: "Config", "Failed to write temp file {}: {e}", tmp_path.display());
        let _ = fs::remove_file(&tmp_path);
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
                quarantine_corrupt_file(&path);
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

    // The profiles directory only ever contains profiles: app-owned files live in the
    // parent `Settings/` directory (see `paths::migrate_profiles_to_subdir`).
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            profiles.push((stem.to_string(), path));
        }
    }

    profiles.sort_by_key(|a| a.0.to_lowercase());
    profiles
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::settings::set_test_settings_dir;

    /// A throw-away settings directory, removed on drop.
    struct TempSettings(PathBuf);

    impl TempSettings {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ntd_profile_{name}_{nanos}"));
            fs::create_dir_all(&path).unwrap();
            log::set_max_level(log::LevelFilter::Trace);
            set_test_settings_dir(path.clone());
            Self(path)
        }
    }

    impl Drop for TempSettings {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn invalid_config() -> MappingConfig {
        let mut config = MappingConfig::default();
        config.active_area.w = -5.0;
        config
    }

    #[test]
    fn sanitizing_removes_reserved_characters_and_traversal() {
        assert_eq!(sanitize_profile_name("osu!"), "osu!");
        assert_eq!(sanitize_profile_name("a/b\\c:d*e?f\"g<h>i|j"), "abcdefghij");
        assert_eq!(sanitize_profile_name("../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_profile_name(".hidden"), "hidden");
        assert_eq!(sanitize_profile_name(""), "unnamed_profile");
        assert_eq!(sanitize_profile_name("..."), "unnamed_profile");
        assert_eq!(sanitize_profile_name("///"), "unnamed_profile");
    }

    #[test]
    fn save_to_path_creates_missing_folders_and_leaves_no_temporary_file() {
        let dir = TempSettings::new("save_path");
        let path = dir.0.join("deep").join("er").join("config.json");
        save_to_path(&path, &MappingConfig::default()).unwrap();
        assert!(path.exists());
        assert!(!path.with_extension("json.tmp").exists());
        let (loaded, corrections) = load_settings_from_file(&path).unwrap();
        assert!(corrections.is_empty());
        assert_eq!(loaded, MappingConfig::default());
    }

    #[test]
    fn save_to_path_reports_a_failure_to_write() {
        let dir = TempSettings::new("save_fail");
        // The target is an existing directory: the final rename cannot succeed.
        let target = dir.0.join("is_a_folder.json");
        fs::create_dir_all(target.join("child")).unwrap();
        assert!(save_to_path(&target, &MappingConfig::default()).is_err());
        assert!(!target.with_extension("json.tmp").exists());
    }

    #[test]
    fn named_presets_get_a_json_extension_once() {
        let dir = TempSettings::new("presets");
        save_settings("osu!", &MappingConfig::default()).unwrap();
        save_settings("Drawing.json", &MappingConfig::default()).unwrap();
        let names: Vec<_> = list_profiles().into_iter().map(|(name, _)| name).collect();
        assert_eq!(names, ["Drawing", "osu!"]);
        assert!(get_profiles_dir().join("osu!.json").exists());
        assert!(get_profiles_dir().join("Drawing.json").exists());
        drop(dir);
    }

    #[test]
    fn profiles_are_listed_case_insensitively_and_junk_is_skipped() {
        let _dir = TempSettings::new("listing");
        for name in ["beta", "Alpha", "gamma"] {
            save_settings(name, &MappingConfig::default()).unwrap();
        }
        fs::write(get_profiles_dir().join("notes.txt"), "not a profile").unwrap();
        let listed = list_profiles();
        let names: Vec<_> = listed.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["Alpha", "beta", "gamma"]);
        assert!(listed.iter().all(|(_, path)| path.exists()));
    }

    #[test]
    fn listing_a_missing_folder_gives_no_profile() {
        let dir = TempSettings::new("no_dir");
        let _ = fs::remove_dir_all(get_profiles_dir());
        assert!(list_profiles().is_empty());
        drop(dir);
    }

    #[test]
    fn there_is_no_last_session_until_one_is_saved() {
        let _dir = TempSettings::new("session");
        assert!(load_last_session().is_none());
        save_last_session(&MappingConfig::default()).unwrap();
        let (config, corrections) = load_last_session().unwrap();
        assert_eq!(config, MappingConfig::default());
        assert!(corrections.is_empty());
    }

    #[test]
    fn an_invalid_last_session_is_repaired_and_saved_back() {
        let dir = TempSettings::new("repair_session");
        save_last_session(&invalid_config()).unwrap();
        let (config, corrections) = load_last_session().unwrap();
        assert!(!corrections.is_empty());
        assert!(config.active_area.w > 0.0);
        // The repaired state replaced the broken file: the next load needs no repair.
        assert!(load_last_session().unwrap().1.is_empty());
        drop(dir);
    }

    #[test]
    fn an_unreadable_last_session_is_quarantined_not_overwritten() {
        let dir = TempSettings::new("corrupt_session");
        let file = dir.0.join("last_session.json");
        fs::write(&file, "{ definitely not json").unwrap();
        assert!(load_last_session().is_none());
        assert!(!file.exists());
        let backup = dir.0.join("last_session.json.corrupt.bak");
        assert_eq!(fs::read_to_string(backup).unwrap(), "{ definitely not json");
    }

    #[test]
    fn loading_a_settings_file_reports_unreadable_and_invalid_files() {
        let dir = TempSettings::new("load_errors");
        assert!(load_settings_from_file(&dir.0.join("missing.json")).is_err());
        let broken = dir.0.join("broken.json");
        fs::write(&broken, "[1, 2").unwrap();
        assert!(load_settings_from_file(&broken).is_err());
    }

    #[test]
    fn loading_a_settings_file_repairs_and_rewrites_it() {
        let dir = TempSettings::new("load_repair");
        let path = dir.0.join("profile.json");
        save_to_path(&path, &invalid_config()).unwrap();
        let (config, corrections) = load_settings_from_file(&path).unwrap();
        assert_eq!(corrections.len(), 1);
        assert!(config.active_area.w > 0.0);
        assert!(load_settings_from_file(&path).unwrap().1.is_empty());
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;

        struct FailingSerialize;

        impl serde::Serialize for FailingSerialize {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("cannot be serialized"))
            }
        }

        #[test]
        fn a_value_that_cannot_be_serialized_is_reported_and_nothing_is_written() {
            let dir = TempSettings::new("serialize_fail");
            let path = dir.0.join("never.json");
            let error = save_json_to_path(&path, &FailingSerialize).unwrap_err();
            assert!(error.contains("cannot be serialized"), "{error}");
            assert!(!path.exists());
        }

        #[test]
        fn a_temporary_file_that_cannot_be_created_is_reported() {
            let dir = TempSettings::new("tmp_blocked");
            let path = dir.0.join("profile.json");
            // The temporary name is taken by a directory.
            fs::create_dir_all(path.with_extension("json.tmp")).unwrap();
            assert!(save_to_path(&path, &MappingConfig::default()).is_err());
            assert!(!path.exists());
        }

        #[test]
        fn quarantining_a_file_that_is_already_gone_only_logs() {
            let dir = TempSettings::new("quarantine_missing");
            quarantine_corrupt_file(&dir.0.join("not_there.json"));
            assert!(!dir.0.join("not_there.json.corrupt.bak").exists());
        }

        #[test]
        fn a_last_session_that_cannot_be_read_is_ignored() {
            let dir = TempSettings::new("session_is_a_folder");
            fs::create_dir_all(dir.0.join("last_session.json")).unwrap();
            assert!(load_last_session().is_none());
        }

        #[test]
        fn a_profiles_folder_that_cannot_be_listed_gives_no_profile() {
            let dir = TempSettings::new("profiles_is_a_file");
            fs::write(dir.0.join("profiles"), "not a folder").unwrap();
            assert!(list_profiles().is_empty());
        }

        #[test]
        fn a_preset_that_cannot_be_saved_is_reported() {
            let dir = TempSettings::new("preset_unsaveable");
            // `profiles` is a file, so nothing can be written inside it.
            fs::write(dir.0.join("profiles"), "not a folder").unwrap();
            assert!(save_settings("osu!", &MappingConfig::default()).is_err());
        }

        #[test]
        fn a_session_that_cannot_be_saved_is_reported() {
            let dir = TempSettings::new("session_unsaveable");
            // The session path is a non-empty directory, so the final rename cannot replace it.
            fs::create_dir_all(dir.0.join("last_session.json").join("child")).unwrap();
            assert!(save_last_session(&MappingConfig::default()).is_err());
        }
    }
}
