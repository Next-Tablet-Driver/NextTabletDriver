//! How a settings file is written and read back.
//!
//! Every file the application persists is a JSON object with a `version` field:
//!
//! - **writes are atomic**: the bytes go to a temporary file next to the target, which is synced
//!   and renamed over it, so a crash never leaves a partial file;
//! - **a file without `version`** comes from v2 and is read as version 0 (the shape did not
//!   change yet; the day it does, the migration is a pure function of version 0);
//! - **a file of a newer version** is refused and left untouched: an older build must not
//!   overwrite what a newer one wrote;
//! - **a corrupted file** is moved to `<name>.corrupt.bak`, never overwritten, and the caller is
//!   told ([`Loaded::Corrupt`]), so a profile is never silently lost.

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The version this build writes into every file.
pub const FORMAT_VERSION: u32 = 1;

/// What reading a settings file gave.
#[derive(Debug)]
pub enum Loaded<T> {
    /// There is no such file.
    Missing,
    /// The file was read.
    Ok {
        /// What the file holds.
        value: T,
        /// The file has no `version`: it was written by v2.
        legacy: bool,
    },
    /// The file is not valid; it was moved aside as `backup` (`None` when even that failed).
    Corrupt {
        /// Why it is not valid.
        reason: String,
        /// Where the file was moved to.
        backup: Option<PathBuf>,
    },
    /// The file was written by a newer version of the application and is left as it is.
    TooNew {
        /// The version found in the file.
        found: u32,
    },
}

/// Why a file could not be written.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The file could not be written or renamed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The value could not be serialised.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The value is not a JSON object, so it cannot carry a `version`.
    #[error("a settings file must be a JSON object")]
    NotAnObject,
}

/// Writes `value` to `path` as a pretty JSON object with the current `version`, atomically.
///
/// # Errors
/// [`StoreError`] when the value cannot be serialised, is not an object, or the file cannot be
/// written; the previous content of `path` is then untouched.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let mut json = serde_json::to_value(value)?;
    let object = json.as_object_mut().ok_or(StoreError::NotAnObject)?;
    object.insert("version".to_string(), FORMAT_VERSION.into());
    write_atomically(path, serde_json::to_string_pretty(&json)?.as_bytes())?;
    Ok(())
}

/// Writes `bytes` to `path` through a temporary file in the same folder.
///
/// # Errors
/// The I/O error; the temporary file is removed and `path` is untouched.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = tmp_path(path);
    let written = fs::File::create(&tmp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            // Synced before the rename, so a power loss cannot leave an empty or partial file.
            file.sync_all()
        })
        .and_then(|()| fs::rename(&tmp, path));
    if let Err(e) = &written {
        log::error!(target: "Config", "Failed to write {}: {e}", path.display());
        let _ = fs::remove_file(&tmp);
    }
    written
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

/// Reads a settings file written by [`write_json`] (or by v2, which has no `version`).
///
/// A file that is not valid JSON of the expected shape is moved aside and reported as
/// [`Loaded::Corrupt`].
#[must_use]
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Loaded<T> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded::Missing,
        Err(e) => {
            // Unreadable is not corrupted: the file may be fine, so it is not moved.
            log::error!(target: "Config", "Failed to read {}: {e}", path.display());
            return Loaded::Corrupt {
                reason: e.to_string(),
                backup: None,
            };
        }
    };
    match parse::<T>(&bytes) {
        Ok(parsed) => parsed,
        Err(reason) => {
            log::error!(target: "Config", "{} is not valid: {reason}", path.display());
            Loaded::Corrupt {
                reason,
                backup: quarantine(path),
            }
        }
    }
}

/// Reads a settings file chosen by the user, which is theirs: it is never moved. A file that is
/// not valid is an error for the caller to show.
///
/// # Errors
/// The reason, as a sentence, when the file is missing, unreadable, invalid or too new.
pub fn read_json_unmoved<T: DeserializeOwned>(path: &Path) -> Result<(T, bool), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    match parse::<T>(&bytes)? {
        Loaded::Ok { value, legacy } => Ok((value, legacy)),
        Loaded::TooNew { found } => Err(too_new(found)),
        Loaded::Missing | Loaded::Corrupt { .. } => Err("the file is empty".to_string()),
    }
}

fn too_new(found: u32) -> String {
    format!("the file is version {found}, newer than this application (version {FORMAT_VERSION})")
}

/// Parses the bytes; `Err` carries the reason when they are not valid.
fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<Loaded<T>, String> {
    let mut json: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let object = json
        .as_object_mut()
        .ok_or_else(|| "the file is not a JSON object".to_string())?;
    let version = match object.remove("version") {
        None => None,
        Some(v) => Some(
            v.as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .ok_or_else(|| "the version is not a number".to_string())?,
        ),
    };
    if let Some(found) = version.filter(|v| *v > FORMAT_VERSION) {
        return Ok(Loaded::TooNew { found });
    }
    let value = serde_json::from_value(json).map_err(|e| e.to_string())?;
    Ok(Loaded::Ok {
        value,
        legacy: version.is_none(),
    })
}

/// Moves `path` to `<path>.corrupt.bak` (or `.corrupt.1.bak`, `.corrupt.2.bak`... when a backup
/// is already there: an earlier backup is never overwritten).
fn quarantine(path: &Path) -> Option<PathBuf> {
    let backup = (0..100).find_map(|n| {
        let suffix = if n == 0 {
            ".corrupt.bak".to_string()
        } else {
            format!(".corrupt.{n}.bak")
        };
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        let candidate = PathBuf::from(name);
        (!candidate.exists()).then_some(candidate)
    })?;
    match fs::rename(path, &backup) {
        Ok(()) => {
            log::warn!(target: "Config", "Invalid settings file preserved as {}", backup.display());
            Some(backup)
        }
        Err(e) => {
            log::error!(
                target: "Config",
                "Failed to back up invalid settings file {}: {e}",
                path.display()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        name: String,
        count: u32,
    }

    fn dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("ntd_store_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn sample() -> Sample {
        Sample {
            name: "a".into(),
            count: 3,
        }
    }

    #[test]
    fn a_written_file_carries_its_version_and_reads_back() {
        let dir = dir("roundtrip");
        let path = dir.join("s.json");
        write_json(&path, &sample()).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(&format!("\"version\": {FORMAT_VERSION}")));
        match read_json::<Sample>(&path) {
            Loaded::Ok { value, legacy } => {
                assert_eq!(value, sample());
                assert!(!legacy);
            }
            other => panic!("{other:?}"),
        }
        // No temporary file is left behind.
        assert!(!tmp_path(&path).exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_file_of_v2_has_no_version_and_is_read_as_legacy() {
        let dir = dir("legacy");
        let path = dir.join("s.json");
        fs::write(&path, r#"{ "name": "old", "count": 1 }"#).unwrap();
        match read_json::<Sample>(&path) {
            Loaded::Ok { value, legacy } => {
                assert_eq!(value.name, "old");
                assert!(legacy);
            }
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_file_is_missing() {
        let dir = dir("missing");
        assert!(matches!(
            read_json::<Sample>(&dir.join("nope.json")),
            Loaded::Missing
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_corrupted_file_is_moved_aside_and_reported_and_no_backup_is_overwritten() {
        let dir = dir("corrupt");
        let path = dir.join("s.json");

        fs::write(&path, "{ not json").unwrap();
        let Loaded::Corrupt { backup, .. } = read_json::<Sample>(&path) else {
            panic!("expected a corrupted file");
        };
        assert_eq!(backup, Some(dir.join("s.json.corrupt.bak")));
        assert!(!path.exists());
        assert_eq!(
            fs::read_to_string(dir.join("s.json.corrupt.bak")).unwrap(),
            "{ not json"
        );

        // A second corruption keeps the first backup.
        fs::write(&path, "[1, 2]").unwrap();
        let Loaded::Corrupt { backup, .. } = read_json::<Sample>(&path) else {
            panic!("expected a corrupted file");
        };
        assert_eq!(backup, Some(dir.join("s.json.corrupt.1.bak")));
        assert_eq!(
            fs::read_to_string(dir.join("s.json.corrupt.bak")).unwrap(),
            "{ not json"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_file_of_the_wrong_shape_is_corrupted_too() {
        let dir = dir("shape");
        let path = dir.join("s.json");
        fs::write(&path, r#"{ "name": 5 }"#).unwrap();
        assert!(matches!(
            read_json::<Sample>(&path),
            Loaded::Corrupt {
                backup: Some(_),
                ..
            }
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_file_of_a_newer_version_is_refused_and_left_alone() {
        let dir = dir("newer");
        let path = dir.join("s.json");
        let text = format!(
            r#"{{ "version": {}, "name": "x", "count": 1 }}"#,
            FORMAT_VERSION + 1
        );
        fs::write(&path, &text).unwrap();
        assert!(matches!(
            read_json::<Sample>(&path),
            Loaded::TooNew { found } if found == FORMAT_VERSION + 1
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_user_file_is_never_moved() {
        let dir = dir("unmoved");
        let path = dir.join("s.json");
        fs::write(&path, "{ not json").unwrap();
        assert!(read_json_unmoved::<Sample>(&path).is_err());
        assert!(path.exists());
        assert!(!dir.join("s.json.corrupt.bak").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_value_that_is_not_an_object_cannot_be_written() {
        let dir = dir("notobject");
        let result = write_json(&dir.join("s.json"), &vec![1, 2, 3]);
        assert!(matches!(result, Err(StoreError::NotAnObject)));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_write_leaves_the_previous_file_untouched() {
        let dir = dir("failed");
        let path = dir.join("s.json");
        write_json(&path, &sample()).unwrap();
        // The temporary file's name is taken by a folder: the write fails.
        fs::create_dir(tmp_path(&path)).unwrap();
        assert!(
            write_json(
                &path,
                &Sample {
                    name: "b".into(),
                    count: 9
                }
            )
            .is_err()
        );
        match read_json::<Sample>(&path) {
            Loaded::Ok { value, .. } => assert_eq!(value, sample()),
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(dir);
    }
}
