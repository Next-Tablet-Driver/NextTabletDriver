//! Profile file I/O: save/load presets and the last-session snapshot.

use crate::paths::Paths;
use crate::store::{self, Loaded, StoreError};
use ntd_core::config::models::MappingConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// Errors produced while reading or writing a profile file.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// The file could not be read, written or renamed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The configuration could not be serialized, or the file is not valid JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The file is not a profile (not valid, or written by a newer version).
    #[error("{0}")]
    Invalid(String),
}

impl From<StoreError> for ProfileError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Io(e) => Self::Io(e),
            StoreError::Json(e) => Self::Json(e),
            StoreError::NotAnObject => Self::Invalid(error.to_string()),
        }
    }
}

/// Atomically writes a `MappingConfig` to an arbitrary path on disk, with its `version`.
///
/// Uses a write-to-temp-then-rename strategy to prevent corruption
/// if the process crashes mid-write.
///
/// # Errors
/// Returns an error if serialization fails, the temporary file cannot be written,
/// or the final rename operation fails.
pub fn save_to_path(path: &Path, config: &MappingConfig) -> Result<(), ProfileError> {
    store::write_json(path, config).map_err(|e| {
        log::error!(target: "Config", "Failed to save config to {}: {e}", path.display());
        ProfileError::from(e)
    })
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
pub fn save_settings(
    paths: &Paths,
    name: &str,
    config: &MappingConfig,
) -> Result<(), ProfileError> {
    let dir = paths.profiles_dir();
    let sanitized_name = sanitize_profile_name(name);

    let filename = if sanitized_name.to_lowercase().ends_with(".json") {
        sanitized_name.clone()
    } else {
        format!("{sanitized_name}.json")
    };

    let path = dir.join(&filename);

    save_to_path(&path, config)?;
    log::info!(target: "Config", "Saved preset '{name}' (sanitized: '{sanitized_name}') to \
        {}", path.display());
    Ok(())
}

fn last_session_path(paths: &Paths) -> PathBuf {
    paths.settings_dir().join("last_session.json")
}

/// Persists the current session state to `last_session.json`.
///
/// Called asynchronously from a background saver thread - never from the UI thread.
///
/// # Errors
/// Returns an error if `save_to_path` fails.
pub fn save_last_session(paths: &Paths, config: &MappingConfig) -> Result<(), ProfileError> {
    save_to_path(&last_session_path(paths), config)?;
    log::trace!(target: "Config", "Last session persistent state updated");
    Ok(())
}

/// The last session config with the fields that had to be repaired.
pub type LastSession = (MappingConfig, Vec<String>);

/// Loads the last session config, running validation and repair on the result, and says what
/// happened to the file.
///
/// - [`Loaded::Missing`]: there is no session file;
/// - [`Loaded::Ok`]: the config and the list of fields that were repaired (empty if all valid);
/// - [`Loaded::Corrupt`]: the file was not valid and was moved aside (`backup`): the caller
///   starts from the defaults and tells the user where the old file is;
/// - [`Loaded::TooNew`]: a newer version wrote it; it is left untouched.
#[must_use]
pub fn load_last_session_outcome(paths: &Paths) -> Loaded<LastSession> {
    let path = last_session_path(paths);
    match store::read_json::<MappingConfig>(&path) {
        Loaded::Ok {
            value: mut config,
            legacy,
        } => {
            let corrections = config.validate_and_repair();
            if !corrections.is_empty() {
                log::warn!(target: "Config", "Last session config had {} field(s) \
                    repaired", corrections.len());
                // Automatically save the repaired config
                let _ = save_last_session(paths, &config);
            }
            log::info!(target: "Config", "Loaded last session from {}", path.display());
            Loaded::Ok {
                value: (config, corrections),
                legacy,
            }
        }
        Loaded::Missing => {
            log::debug!(target: "Config", "No last session file found at {}", path.display());
            Loaded::Missing
        }
        Loaded::Corrupt { reason, backup } => Loaded::Corrupt { reason, backup },
        Loaded::TooNew { found } => {
            log::error!(target: "Config", "The last session was written by a newer version \
                ({found}) and is left as it is");
            Loaded::TooNew { found }
        }
    }
}

/// Loads the last session config, running validation and repair on the result.
///
/// Returns `None` if no usable session exists (see [`load_last_session_outcome`] for the reason).
/// Returns `Some((config, corrections))` where `corrections` is a list of fields that were
/// repaired (empty if all valid).
#[must_use]
pub fn load_last_session(paths: &Paths) -> Option<LastSession> {
    match load_last_session_outcome(paths) {
        Loaded::Ok { value, .. } => Some(value),
        Loaded::Missing | Loaded::Corrupt { .. } | Loaded::TooNew { .. } => None,
    }
}

/// Loads and validates a config from an arbitrary file path chosen by the user. The file is
/// never moved, whatever it holds.
///
/// Returns the config and a list of corrections applied during validation.
///
/// # Errors
/// Returns an error if the file cannot be read, is not a valid profile, or was written by a newer
/// version.
pub fn load_settings_from_file(path: &Path) -> Result<LastSession, ProfileError> {
    let (mut config, _legacy) = store::read_json_unmoved::<MappingConfig>(path).map_err(|e| {
        log::error!(target: "Config", "Failed to load settings file {}: {e}", path.display());
        ProfileError::Invalid(e)
    })?;

    let corrections = config.validate_and_repair();

    if !corrections.is_empty() {
        log::warn!(target: "Config", "Config from {} had {} field(s) \
            repaired", path.display(), corrections.len());
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
pub fn list_profiles(paths: &Paths) -> Vec<(String, PathBuf)> {
    let dir = paths.profiles_dir();
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
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;

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
            let root = std::env::temp_dir().join(format!("ntd_profile_{name}_{nanos}"));
            fs::create_dir_all(&root).unwrap();
            let paths = Paths::new(root.clone());
            Self { root, paths }
        }
    }

    impl Drop for TempSettings {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
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
        let path = dir.root.join("deep").join("er").join("config.json");
        save_to_path(&path, &MappingConfig::default()).unwrap();
        assert!(path.exists());
        assert!(!dir.root.join("deep/er/config.json.tmp").exists());
        let (loaded, corrections) = load_settings_from_file(&path).unwrap();
        assert!(corrections.is_empty());
        assert_eq!(loaded, MappingConfig::default());
    }

    #[test]
    fn a_saved_profile_carries_its_version() {
        let dir = TempSettings::new("versioned");
        let path = dir.root.join("config.json");
        save_to_path(&path, &MappingConfig::default()).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"version\": 1"), "{text}");
    }

    #[test]
    fn a_profile_of_v2_without_a_version_still_loads() {
        let dir = TempSettings::new("v2_profile");
        let path = dir.root.join("v2.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&MappingConfig::default()).unwrap(),
        )
        .unwrap();
        let (loaded, corrections) = load_settings_from_file(&path).unwrap();
        assert!(corrections.is_empty());
        assert_eq!(loaded, MappingConfig::default());
    }

    #[test]
    fn a_profile_of_a_newer_version_is_refused_and_not_modified() {
        let dir = TempSettings::new("newer_profile");
        let path = dir.root.join("future.json");
        let mut value = serde_json::to_value(MappingConfig::default()).unwrap();
        value["version"] = serde_json::json!(99);
        let text = serde_json::to_string(&value).unwrap();
        fs::write(&path, &text).unwrap();
        let error = load_settings_from_file(&path).unwrap_err();
        assert!(error.to_string().contains("newer"), "{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn save_to_path_reports_a_failure_to_write() {
        let dir = TempSettings::new("save_fail");
        // The target is an existing directory: the final rename cannot succeed.
        let target = dir.root.join("is_a_folder.json");
        fs::create_dir_all(target.join("child")).unwrap();
        assert!(save_to_path(&target, &MappingConfig::default()).is_err());
        assert!(!dir.root.join("is_a_folder.json.tmp").exists());
    }

    #[test]
    fn named_presets_get_a_json_extension_once() {
        let dir = TempSettings::new("presets");
        save_settings(&dir.paths, "osu!", &MappingConfig::default()).unwrap();
        save_settings(&dir.paths, "Drawing.json", &MappingConfig::default()).unwrap();
        let names: Vec<_> = list_profiles(&dir.paths)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, ["Drawing", "osu!"]);
        assert!(dir.paths.profiles_dir().join("osu!.json").exists());
        assert!(dir.paths.profiles_dir().join("Drawing.json").exists());
    }

    #[test]
    fn profiles_are_listed_case_insensitively_and_junk_is_skipped() {
        let dir = TempSettings::new("listing");
        for name in ["beta", "Alpha", "gamma"] {
            save_settings(&dir.paths, name, &MappingConfig::default()).unwrap();
        }
        fs::write(dir.paths.profiles_dir().join("notes.txt"), "not a profile").unwrap();
        let listed = list_profiles(&dir.paths);
        let names: Vec<_> = listed.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["Alpha", "beta", "gamma"]);
        assert!(listed.iter().all(|(_, path)| path.exists()));
    }

    #[test]
    fn listing_a_missing_folder_gives_no_profile() {
        let dir = TempSettings::new("no_dir");
        let _ = fs::remove_dir_all(dir.paths.profiles_dir());
        // The folder is created again on demand, and is empty.
        assert!(list_profiles(&dir.paths).is_empty());
    }

    #[test]
    fn there_is_no_last_session_until_one_is_saved() {
        let dir = TempSettings::new("session");
        assert!(load_last_session(&dir.paths).is_none());
        save_last_session(&dir.paths, &MappingConfig::default()).unwrap();
        let (config, corrections) = load_last_session(&dir.paths).unwrap();
        assert_eq!(config, MappingConfig::default());
        assert!(corrections.is_empty());
    }

    #[test]
    fn an_invalid_last_session_is_repaired_and_saved_back() {
        let dir = TempSettings::new("repair_session");
        save_last_session(&dir.paths, &invalid_config()).unwrap();
        let (config, corrections) = load_last_session(&dir.paths).unwrap();
        assert!(!corrections.is_empty());
        assert!(config.active_area.w > 0.0);
        // The repaired state replaced the broken file: the next load needs no repair.
        assert!(load_last_session(&dir.paths).unwrap().1.is_empty());
    }

    #[test]
    fn a_corrupted_last_session_is_backed_up_reported_and_not_overwritten() {
        let dir = TempSettings::new("corrupt_session");
        let file = dir.root.join("last_session.json");
        fs::write(&file, "{ definitely not json").unwrap();

        let outcome = load_last_session_outcome(&dir.paths);
        let Loaded::Corrupt { backup, reason } = outcome else {
            panic!("expected a corrupted file");
        };
        assert!(!reason.is_empty());
        assert_eq!(backup, Some(dir.root.join("last_session.json.corrupt.bak")));
        assert!(!file.exists());
        assert_eq!(
            fs::read_to_string(dir.root.join("last_session.json.corrupt.bak")).unwrap(),
            "{ definitely not json"
        );
        // The start is from the defaults, and the next save does not touch the backup.
        assert!(load_last_session(&dir.paths).is_none());
    }

    #[test]
    fn a_last_session_of_a_newer_version_is_left_alone() {
        let dir = TempSettings::new("newer_session");
        let file = dir.root.join("last_session.json");
        let mut value = serde_json::to_value(MappingConfig::default()).unwrap();
        value["version"] = serde_json::json!(42);
        let text = serde_json::to_string(&value).unwrap();
        fs::write(&file, &text).unwrap();
        assert!(matches!(
            load_last_session_outcome(&dir.paths),
            Loaded::TooNew { found: 42 }
        ));
        assert_eq!(fs::read_to_string(&file).unwrap(), text);
    }

    #[test]
    fn loading_a_settings_file_reports_unreadable_and_invalid_files_without_moving_them() {
        let dir = TempSettings::new("load_errors");
        assert!(load_settings_from_file(&dir.root.join("missing.json")).is_err());
        let broken = dir.root.join("broken.json");
        fs::write(&broken, "[1, 2").unwrap();
        assert!(load_settings_from_file(&broken).is_err());
        assert!(broken.exists());
    }

    #[test]
    fn loading_a_settings_file_repairs_and_rewrites_it() {
        let dir = TempSettings::new("load_repair");
        let path = dir.root.join("profile.json");
        save_to_path(&path, &invalid_config()).unwrap();
        let (config, corrections) = load_settings_from_file(&path).unwrap();
        assert_eq!(corrections.len(), 1);
        assert!(config.active_area.w > 0.0);
        assert!(load_settings_from_file(&path).unwrap().1.is_empty());
    }

    #[test]
    fn a_temporary_file_that_cannot_be_created_is_reported() {
        let dir = TempSettings::new("tmp_blocked");
        let path = dir.root.join("profile.json");
        // The temporary name is taken by a directory.
        fs::create_dir_all(dir.root.join("profile.json.tmp")).unwrap();
        assert!(save_to_path(&path, &MappingConfig::default()).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn a_last_session_that_cannot_be_read_is_ignored() {
        let dir = TempSettings::new("session_is_a_folder");
        fs::create_dir_all(dir.root.join("last_session.json")).unwrap();
        assert!(load_last_session(&dir.paths).is_none());
    }

    #[test]
    fn a_profiles_folder_that_cannot_be_listed_gives_no_profile() {
        let dir = TempSettings::new("profiles_is_a_file");
        fs::write(dir.root.join("profiles"), "not a folder").unwrap();
        assert!(list_profiles(&dir.paths).is_empty());
    }

    #[test]
    fn a_preset_that_cannot_be_saved_is_reported() {
        let dir = TempSettings::new("preset_unsaveable");
        // `profiles` is a file, so nothing can be written inside it.
        fs::write(dir.root.join("profiles"), "not a folder").unwrap();
        assert!(save_settings(&dir.paths, "osu!", &MappingConfig::default()).is_err());
    }

    #[test]
    fn a_session_that_cannot_be_saved_is_reported() {
        let dir = TempSettings::new("session_unsaveable");
        // The session path is a non-empty directory, so the final rename cannot replace it.
        fs::create_dir_all(dir.root.join("last_session.json").join("child")).unwrap();
        assert!(save_last_session(&dir.paths, &MappingConfig::default()).is_err());
    }
}
