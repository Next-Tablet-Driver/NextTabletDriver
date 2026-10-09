//! Lightweight active-profile identity, persisted across restarts.

use super::paths::get_settings_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    match serde_json::to_string_pretty(meta) {
        Ok(json) => {
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = fs::write(&tmp, &json) {
                log::error!(target: "Config", "Failed to write temp session meta: {e}");
            } else if let Err(e) = fs::rename(&tmp, &path) {
                log::error!(target: "Config", "Failed to rename temp session meta: {e}");
            }
        }
        Err(e) => {
            log::error!(target: "Config", "Failed to serialize session meta: {e}");
        }
    }
}

/// Loads the active profile metadata from `session_meta.json`.
#[must_use]
pub fn load_session_meta() -> Option<SessionMeta> {
    let path = get_settings_dir().join("session_meta.json");
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(meta) => Some(meta),
            Err(e) => {
                log::error!(target: "Config", "Failed to parse session meta: {e}");
                None
            }
        },
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::error!(target: "Config", "Failed to read session meta: {e}");
            }
            None
        }
    }
}
