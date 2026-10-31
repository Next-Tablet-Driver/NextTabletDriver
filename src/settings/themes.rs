//! User theme files (`Settings/Themes/*.json`).
//!
//! A theme is a data file, validated in depth by the frontend before it
//! is applied. This module only deals with the files: listing, importing and deleting them,
//! with limits on size and count, and without ever using a path supplied by the webview.

use super::paths::get_settings_dir;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Largest theme file accepted, in bytes (the frontend enforces the same limit).
pub const MAX_THEME_BYTES: u64 = 64 * 1024;
/// Most theme files listed.
pub const MAX_THEME_FILES: usize = 100;

/// A theme file as handed to the frontend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ThemeFile {
    /// Stable id derived from the file name; what `custom:<id>` refers to.
    pub id: String,
    pub file_name: String,
    /// Raw file content, parsed and validated by the frontend.
    pub content: String,
}

/// The directory user themes live in, created on demand.
#[must_use]
pub fn themes_dir() -> PathBuf {
    let dir = get_settings_dir().join("Themes");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Derives a theme id from a file stem: lowercase `a-z0-9_-`, at most 64 characters.
///
/// Returns `None` if nothing usable is left.
#[must_use]
pub fn theme_id_from_stem(stem: &str) -> Option<String> {
    let id: String = stem
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() { None } else { Some(id) }
}

fn is_json_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// Errors produced while listing, importing or deleting a theme file.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    /// The file name is not valid UTF-8.
    #[error("Invalid theme file name")]
    InvalidFileName,
    /// The file name contains no character that can be used in a theme id.
    #[error("Theme file name has no usable characters")]
    NoUsableName,
    /// The file is bigger than the accepted size.
    #[error("{file_name} is larger than {max_kb} KB")]
    TooLarge {
        /// Name of the offending file.
        file_name: String,
        /// The size limit, in kilobytes.
        max_kb: u64,
    },
    /// The file could not be read.
    #[error("{file_name}: {source}")]
    Read {
        /// Name of the file.
        file_name: String,
        /// Why it could not be read.
        #[source]
        source: std::io::Error,
    },
    /// The file or the themes folder could not be inspected.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The chosen file does not have the `.json` extension.
    #[error("A theme must be a .json file")]
    NotJsonFile,
    /// The file is valid JSON but not a JSON object.
    #[error("A theme file must contain a JSON object")]
    NotAnObject,
    /// The file is not valid JSON.
    #[error("Not valid JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    /// The copy into the themes folder failed.
    #[error("Failed to save the theme: {0}")]
    Save(#[source] std::io::Error),
    /// No theme has this id.
    #[error("No theme with id '{id}'")]
    NotFound {
        /// The id that was asked for.
        id: String,
    },
    /// The theme file could not be removed.
    #[error("Failed to delete the theme: {0}")]
    Delete(#[source] std::io::Error),
}

fn read_theme(path: &Path) -> Result<ThemeFile, ThemeError> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(ThemeError::InvalidFileName)?
        .to_string();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let id = theme_id_from_stem(stem).ok_or(ThemeError::NoUsableName)?;

    let size = fs::metadata(path)?.len();
    if size > MAX_THEME_BYTES {
        return Err(ThemeError::TooLarge {
            file_name,
            max_kb: MAX_THEME_BYTES / 1024,
        });
    }
    let content = fs::read_to_string(path).map_err(|source| ThemeError::Read {
        file_name: file_name.clone(),
        source,
    })?;
    Ok(ThemeFile {
        id,
        file_name,
        content,
    })
}

/// Lists the theme files, sorted by file name. Unreadable, oversized or duplicate-id files are
/// skipped (and logged) so one bad file never hides the others.
#[must_use]
pub fn list_theme_files() -> Vec<ThemeFile> {
    let Ok(entries) = fs::read_dir(themes_dir()) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_json_file(p))
        .collect();
    paths.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
    });

    let mut themes: Vec<ThemeFile> = Vec::new();
    for path in paths.into_iter().take(MAX_THEME_FILES) {
        match read_theme(&path) {
            Ok(theme) if themes.iter().any(|t| t.id == theme.id) => {
                log::warn!(target: "Theme", "Skipping {}: another theme already uses the id '{}'", theme.file_name, theme.id);
            }
            Ok(theme) => themes.push(theme),
            Err(e) => log::warn!(target: "Theme", "Skipping theme file {}: {e}", path.display()),
        }
    }
    themes
}

/// Copies a theme file chosen by the user into the themes folder.
///
/// The source must be a `.json` file under the size limit containing a JSON object; deeper
/// validation happens in the frontend. An existing theme with the same id is never
/// overwritten: the copy gets a numeric suffix.
///
/// # Errors
/// Returns an error if the file is not acceptable or cannot be copied.
pub fn import_theme_file(source: &Path) -> Result<ThemeFile, ThemeError> {
    if !is_json_file(source) {
        return Err(ThemeError::NotJsonFile);
    }
    let imported = read_theme(source)?;
    match serde_json::from_str::<serde_json::Value>(&imported.content) {
        Ok(value) if value.is_object() => {}
        Ok(_) => return Err(ThemeError::NotAnObject),
        Err(e) => return Err(ThemeError::InvalidJson(e)),
    }

    let dir = themes_dir();
    let existing: Vec<String> = list_theme_files().into_iter().map(|t| t.id).collect();
    let mut id = imported.id.clone();
    let mut n = 2;
    while existing.contains(&id) || dir.join(format!("{id}.json")).exists() {
        id = format!("{}-{n}", imported.id);
        n += 1;
    }

    let file_name = format!("{id}.json");
    fs::write(dir.join(&file_name), &imported.content).map_err(ThemeError::Save)?;
    Ok(ThemeFile {
        id,
        file_name,
        content: imported.content,
    })
}

/// Deletes the theme with this id.
///
/// The file is found by listing the themes folder, so the id is never turned into a path.
///
/// # Errors
/// Returns an error if no theme has this id or the file cannot be removed.
pub fn delete_theme_file(id: &str) -> Result<(), ThemeError> {
    let target = fs::read_dir(themes_dir())?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            is_json_file(p)
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(theme_id_from_stem)
                    .is_some_and(|candidate| candidate == id)
        })
        .ok_or_else(|| ThemeError::NotFound { id: id.to_string() })?;
    fs::remove_file(target).map_err(ThemeError::Delete)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::set_test_settings_dir;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let path = std::env::temp_dir().join(format!("ntd_themes_{name}_{nanos}"));
            fs::create_dir_all(&path).unwrap();
            set_test_settings_dir(path.clone());
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const VALID: &str = r#"{"schema":1,"metadata":{"name":"X"},"base":"dark","tokens":{}}"#;

    #[test]
    fn ids_are_lowercase_and_safe() {
        assert_eq!(theme_id_from_stem("My Theme").as_deref(), Some("my-theme"));
        assert_eq!(
            theme_id_from_stem("neon_night-2").as_deref(),
            Some("neon_night-2")
        );
        assert_eq!(
            theme_id_from_stem("../../etc/passwd").as_deref(),
            Some("etc-passwd")
        );
        assert_eq!(theme_id_from_stem("  ").as_deref(), None);
        assert_eq!(theme_id_from_stem("///").as_deref(), None);
        assert_eq!(theme_id_from_stem(&"a".repeat(200)).unwrap().len(), 64);
    }

    #[test]
    fn lists_json_files_sorted_and_skips_junk() {
        let _dir = TempDir::new("list");
        let themes = themes_dir();
        fs::write(themes.join("b.json"), VALID).unwrap();
        fs::write(themes.join("A.json"), VALID).unwrap();
        fs::write(themes.join("notes.txt"), "x").unwrap();
        fs::write(
            themes.join("huge.json"),
            vec![b' '; (MAX_THEME_BYTES + 1) as usize],
        )
        .unwrap();

        let listed = list_theme_files();
        let names: Vec<_> = listed.iter().map(|t| t.file_name.as_str()).collect();
        assert_eq!(names, vec!["A.json", "b.json"]);
        assert_eq!(listed.first().map(|t| t.id.as_str()), Some("a"));
    }

    #[test]
    fn colliding_ids_keep_the_first_file() {
        let _dir = TempDir::new("collide");
        let themes = themes_dir();
        fs::write(themes.join("My Theme.json"), VALID).unwrap();
        fs::write(themes.join("my-theme.json"), VALID).unwrap();
        assert_eq!(list_theme_files().len(), 1);
    }

    #[test]
    fn import_copies_valid_json_and_never_overwrites() {
        let dir = TempDir::new("import");
        let source = dir.0.join("incoming.json");
        fs::write(&source, VALID).unwrap();

        let first = import_theme_file(&source).unwrap();
        assert_eq!(first.id, "incoming");
        let second = import_theme_file(&source).unwrap();
        assert_eq!(second.id, "incoming-2");
        assert_eq!(list_theme_files().len(), 2);
    }

    #[test]
    fn import_rejects_non_themes() {
        let dir = TempDir::new("reject");
        let not_json = dir.0.join("a.json");
        fs::write(&not_json, "{ nope").unwrap();
        assert!(matches!(
            import_theme_file(&not_json).unwrap_err(),
            ThemeError::InvalidJson(_)
        ));

        let array = dir.0.join("b.json");
        fs::write(&array, "[]").unwrap();
        assert!(matches!(
            import_theme_file(&array).unwrap_err(),
            ThemeError::NotAnObject
        ));

        let wrong_ext = dir.0.join("c.txt");
        fs::write(&wrong_ext, VALID).unwrap();
        assert!(matches!(
            import_theme_file(&wrong_ext).unwrap_err(),
            ThemeError::NotJsonFile
        ));

        let huge = dir.0.join("d.json");
        fs::write(&huge, vec![b' '; (MAX_THEME_BYTES + 1) as usize]).unwrap();
        assert!(matches!(
            import_theme_file(&huge).unwrap_err(),
            ThemeError::TooLarge { .. }
        ));
        assert!(list_theme_files().is_empty());
    }

    #[test]
    fn delete_finds_the_file_by_id_and_ignores_path_tricks() {
        let _dir = TempDir::new("delete");
        let themes = themes_dir();
        fs::write(themes.join("Neon Night.json"), VALID).unwrap();
        let outside = themes.parent().unwrap().join("secret.json");
        fs::write(&outside, VALID).unwrap();

        assert!(delete_theme_file("../secret").is_err());
        assert!(outside.exists());
        assert!(delete_theme_file("missing").is_err());

        delete_theme_file("neon-night").unwrap();
        assert!(list_theme_files().is_empty());
    }

    #[test]
    fn a_themes_folder_that_cannot_be_listed_gives_no_theme() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ntd_themes_blocked_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        crate::settings::set_test_settings_dir(dir.clone());
        // "Themes" is a regular file, so it is neither created nor listed.
        fs::write(dir.join("Themes"), "not a folder").unwrap();
        assert!(list_theme_files().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    fn blocked_settings(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ntd_themes_{name}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        crate::settings::set_test_settings_dir(dir.clone());
        dir
    }

    #[test]
    fn a_theme_path_without_a_file_name_is_refused() {
        assert!(matches!(
            read_theme(Path::new("..")).unwrap_err(),
            ThemeError::InvalidFileName
        ));
    }

    #[test]
    fn a_theme_name_without_usable_characters_is_refused() {
        assert!(matches!(
            read_theme(Path::new("!!!.json")).unwrap_err(),
            ThemeError::NoUsableName
        ));
    }

    #[test]
    fn a_theme_file_that_does_not_exist_is_refused() {
        assert!(read_theme(Path::new("ntd-no-such-theme.json")).is_err());
    }

    #[test]
    fn a_theme_file_that_is_not_text_is_refused() {
        let dir = blocked_settings("binary");
        let file = dir.join("binary.json");
        fs::write(&file, [0xFF, 0xFE, 0xFD]).unwrap();
        let error = read_theme(&file).unwrap_err();
        assert!(
            matches!(&error, ThemeError::Read { file_name, .. } if file_name == "binary.json"),
            "{error}"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn listing_skips_duplicate_ids_and_unreadable_files() {
        crate::test_support::evaluate_log_arguments();
        let dir = blocked_settings("listing");
        let themes = dir.join("Themes");
        fs::create_dir_all(&themes).unwrap();
        fs::write(themes.join("Cool Theme.json"), "{}").unwrap();
        fs::write(themes.join("cool-theme.json"), "{}").unwrap();
        fs::write(themes.join("broken.json"), [0xFF, 0xFE]).unwrap();
        let listed = list_theme_files();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "cool-theme");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_theme_that_cannot_be_written_is_reported() {
        let dir = blocked_settings("unwritable");
        let source = dir.join("source.json");
        fs::write(&source, "{}").unwrap();
        // `Themes` is a file, so the copy has nowhere to go.
        fs::write(dir.join("Themes"), "not a folder").unwrap();
        let error = import_theme_file(&source).unwrap_err();
        assert!(matches!(error, ThemeError::Save(_)), "{error}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn deleting_from_a_themes_folder_that_cannot_be_read_is_reported() {
        let dir = blocked_settings("undeletable_folder");
        fs::write(dir.join("Themes"), "not a folder").unwrap();
        assert!(delete_theme_file("neon").is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_theme_that_cannot_be_removed_is_reported() {
        let dir = blocked_settings("undeletable_theme");
        // A directory that looks like a theme file cannot be removed as a file.
        fs::create_dir_all(dir.join("Themes").join("neon.json")).unwrap();
        let error = delete_theme_file("neon").unwrap_err();
        assert!(matches!(error, ThemeError::Delete(_)), "{error}");
        let _ = fs::remove_dir_all(dir);
    }
}
