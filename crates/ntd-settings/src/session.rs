//! Lightweight active-profile identity, persisted across restarts.

use crate::paths::Paths;
use crate::store::{self, Loaded};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Lightweight metadata about the active profile, persisted across restarts.
///
/// Stored in `session_meta.json` alongside `last_session.json`.
/// This allows the driver to silently restore the profile identity
/// (name + file path) on startup without re-prompting the user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMeta {
    /// Display name of the active profile.
    pub profile_name: String,
    /// Absolute path to the profile file on disk, if any.
    pub profile_path: Option<PathBuf>,
}

fn meta_path(paths: &Paths) -> PathBuf {
    paths.settings_dir().join("session_meta.json")
}

/// Saves the active profile metadata to `session_meta.json`.
pub fn save_session_meta(paths: &Paths, meta: &SessionMeta) {
    if let Err(e) = store::write_json(&meta_path(paths), meta) {
        log::error!(target: "Config", "Failed to save session meta: {e}");
    }
}

/// Loads the active profile metadata from `session_meta.json`. A file that is not valid is moved
/// aside (see [`store::read_json`]) and there is no active profile.
#[must_use]
pub fn load_session_meta(paths: &Paths) -> Option<SessionMeta> {
    load_session_meta_from(&meta_path(paths))
}

fn load_session_meta_from(path: &Path) -> Option<SessionMeta> {
    match store::read_json::<SessionMeta>(path) {
        Loaded::Ok { value, .. } => Some(value),
        Loaded::Missing => None,
        Loaded::Corrupt { .. } | Loaded::TooNew { .. } => {
            log::error!(target: "Config", "Session meta {} is not usable", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn paths(name: &str) -> Paths {
        let root = std::env::temp_dir().join(format!("ntd_session_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Paths::new(root)
    }

    #[test]
    fn the_session_meta_round_trips_with_its_version() {
        let paths = paths("roundtrip");
        let meta = SessionMeta {
            profile_name: "Default Profile".to_string(),
            profile_path: Some(PathBuf::from("C:\\some\\path.json")),
        };
        save_session_meta(&paths, &meta);

        let text = fs::read_to_string(paths.settings_dir().join("session_meta.json")).unwrap();
        assert!(text.contains("\"version\": 1"), "{text}");
        assert_eq!(load_session_meta(&paths), Some(meta));
        let _ = fs::remove_dir_all(paths.settings_dir());
    }

    #[test]
    fn a_session_meta_of_v2_without_a_version_still_loads() {
        let paths = paths("legacy");
        fs::write(
            paths.settings_dir().join("session_meta.json"),
            r#"{ "profile_name": "Old", "profile_path": null }"#,
        )
        .unwrap();
        let loaded = load_session_meta(&paths).unwrap();
        assert_eq!(loaded.profile_name, "Old");
        let _ = fs::remove_dir_all(paths.settings_dir());
    }

    #[test]
    fn a_corrupted_session_meta_is_backed_up_and_gives_no_session() {
        let paths = paths("corrupt");
        let file = paths.settings_dir().join("session_meta.json");
        fs::write(&file, "{ nope").unwrap();
        assert!(load_session_meta(&paths).is_none());
        assert!(!file.exists());
        assert_eq!(
            fs::read_to_string(paths.settings_dir().join("session_meta.json.corrupt.bak")).unwrap(),
            "{ nope"
        );
        let _ = fs::remove_dir_all(paths.settings_dir());
    }

    #[test]
    fn there_is_no_session_meta_until_one_is_saved() {
        let paths = paths("missing");
        assert!(load_session_meta(&paths).is_none());
        let _ = fs::remove_dir_all(paths.settings_dir());
    }
}
