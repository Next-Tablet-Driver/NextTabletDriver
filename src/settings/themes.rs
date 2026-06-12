use crate::core::config::theme_models::ThemeConfig;
use std::fs;
use std::path::PathBuf;

/// Gets the path to the local themes directory. Creates it if it doesn't exist.
#[must_use]
pub fn get_themes_dir() -> PathBuf {
    let dir = crate::settings::get_settings_dir().join("themes");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Lists all available custom themes in the themes directory.
/// Returns a list of folder names.
#[must_use]
pub fn list_custom_themes() -> Vec<String> {
    let dir = get_themes_dir();
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                // Check if it has a theme.json
                if path.join("theme.json").exists() {
                    themes.push(name.to_string());
                }
            }
        }
    }
    themes
}

/// Loads a custom theme configuration from its folder name.
#[must_use]
pub fn load_custom_theme(name: &str) -> Option<ThemeConfig> {
    let path = get_themes_dir().join(name).join("theme.json");
    let res = fs::read_to_string(path).map_or(None, |content| serde_json::from_str(&content).ok());
    if res.is_some() {
        log::info!(target: "Theme", "Successfully loaded custom theme '{name}'");
    } else {
        log::warn!(target: "Theme", "Failed to load custom theme '{name}' or file is invalid");
    }
    res
}

/// Imports a new theme from a JSON file path.
/// It creates a folder using the sanitized theme name and copies the JSON.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn import_theme_json(source_path: &std::path::Path) -> Result<String, String> {
    let content = fs::read_to_string(source_path).map_err(|e| e.to_string())?;
    import_theme_from_string(&content)
}

/// Imports a new theme directly from a JSON string.
/// It creates a folder using the sanitized theme name and writes the JSON.
///
/// # Errors
///
/// Returns an error if the string is invalid JSON or cannot be written to disk.
pub fn import_theme_from_string(content: &str) -> Result<String, String> {
    // Validate that it is a proper ThemeConfig
    let config: ThemeConfig = serde_json::from_str(content).map_err(|e| {
        log::error!(target: "Theme", "Failed to parse theme JSON: {e}");
        format!("Invalid theme format: {e}")
    })?;

    let safe_name = crate::settings::sanitize_profile_name(&config.metadata.name);
    let theme_folder = get_themes_dir().join(&safe_name);

    if !theme_folder.exists() {
        fs::create_dir_all(&theme_folder).map_err(|e| {
            log::error!(target: "Theme", "Failed to create directory {}: {e}", theme_folder.display());
            e.to_string()
        })?;
    }

    fs::write(theme_folder.join("theme.json"), content).map_err(|e| {
        log::error!(target: "Theme", "Failed to write theme.json in {}: {e}", theme_folder.display());
        e.to_string()
    })?;
    log::info!(target: "Theme", "Successfully imported custom theme '{}' as '{}'", config.metadata.name, safe_name);

    Ok(safe_name)
}

/// Deletes a custom theme and its folder.
///
/// # Errors
///
/// Returns an error if the theme does not exist or cannot be deleted.
pub fn delete_custom_theme(name: &str) -> Result<(), String> {
    let theme_folder = get_themes_dir().join(name);
    if theme_folder.exists() && theme_folder.is_dir() {
        fs::remove_dir_all(theme_folder).map_err(|e| e.to_string())?;
        log::info!(target: "Theme", "Successfully deleted custom theme '{name}'");
    }
    Ok(())
}

use crate::app::state::ThemeStoreItem;
use crate::core::config::theme_models::ThemeMetadata;

/// Fetches the list of themes from the GitHub repository synchronously.
///
/// # Errors
/// Returns an error if the network request fails, or if the API response is invalid JSON.
pub fn fetch_theme_store_list_sync() -> Result<Vec<ThemeStoreItem>, String> {
    let url = "https://api.github.com/repos/Next-Tablet-Driver/NextTabletDriver-Themes/contents/";
    match ureq::get(url).call() {
        Ok(response) => response.into_json::<serde_json::Value>().map_or_else(
            |_| Err("Failed to parse JSON".to_string()),
            |json| {
                json.as_array().map_or_else(
                    || Err("Invalid API response".to_string()),
                    |arr| {
                        let mut themes = Vec::new();
                        for item in arr {
                            if item["type"].as_str() == Some("dir")
                                && let Some(name) = item["name"].as_str()
                                && name != "00 EXAMPLE"
                                && name != ".github"
                            {
                                let encoded_name = name.replace(' ', "%20");
                                let theme_url = format!("https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver-Themes/refs/heads/main/{encoded_name}/theme.json");
                                if let Ok(res) = ureq::get(&theme_url).call()
                                    && let Ok(content) = res.into_string()
                                {
                                    if let Ok(config) = serde_json::from_str::<ThemeConfig>(&content) {
                                        themes.push(ThemeStoreItem {
                                            metadata: config.metadata,
                                            dark_mode: config.colors.dark_mode,
                                        });
                                    } else {
                                        log::error!(target: "ThemeStore", "Failed to parse theme.json for {name}");
                                        themes.push(ThemeStoreItem {
                                            metadata: ThemeMetadata {
                                                name: name.to_string(),
                                                author: "Unknown".to_string(),
                                                version: "1.0".to_string(),
                                                update_url: None,
                                            },
                                            dark_mode: true,
                                        });
                                    }
                                } else {
                                    log::error!(target: "ThemeStore", "Failed to download theme.json for {name}");
                                }
                            }
                        }
                        Ok(themes)
                    },
                )
            },
        ),
        Err(e) => Err(format!("Network error: {e}")),
    }
}

/// Downloads and installs a theme synchronously from GitHub.
///
/// # Errors
/// Returns an error if the network request fails or if the theme content cannot be parsed.
pub fn download_and_install_theme_sync(theme: &str) -> Result<String, String> {
    let encoded_theme = theme.replace(' ', "%20");
    let url = format!(
        "https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver-Themes/refs/heads/main/{encoded_theme}/theme.json"
    );
    match ureq::get(&url).call() {
        Ok(response) => response.into_string().map_or_else(
            |e| {
                log::error!(target: "ThemeStore", "Failed to read response content: {e}");
                Err("Failed to read response content".into())
            },
            |content| import_theme_from_string(&content),
        ),
        Err(e) => {
            log::error!(target: "ThemeStore", "Network error while downloading theme '{theme}': {e}");
            Err(format!("Network error: {e}"))
        }
    }
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
            let path = std::env::temp_dir().join(format!("ntd_theme_tests_{name}_{nanos}"));
            fs::create_dir_all(&path).unwrap();
            crate::settings::set_test_settings_dir(path.clone());
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_import_and_load_theme() {
        let _guard = TempDirGuard::new("import_load");

        let json_theme = r##"{
            "metadata": {
                "name": "Test Theme",
                "author": "Author",
                "version": "1.0.0",
                "update_url": null
            },
            "colors": {
                "dark_mode": true,
                "panel_bg": "#111111",
                "window_bg": "#222222",
                "text_color": "#333333",
                "strong_text_color": "#444444",
                "accent_color": "#555555",
                "border_color": "#666666",
                "widget_bg": "#777777",
                "widget_hover": "#888888",
                "widget_active": "#999999",
                "success_color": null,
                "warning_color": null,
                "error_color": null,
                "info_color": null,
                "playfield_color": null,
                "playfield_opacity": null
            },
            "spacing": null
        }"##;

        // Import theme from string
        let import_res = import_theme_from_string(json_theme);
        assert!(import_res.is_ok());
        let safe_name = import_res.unwrap();
        assert_eq!(safe_name, "Test Theme");

        // Verify folder structure exists
        let expected_dir = get_themes_dir().join(&safe_name);
        assert!(expected_dir.exists());
        assert!(expected_dir.join("theme.json").exists());

        // List custom themes
        let themes = list_custom_themes();
        assert_eq!(themes, vec!["Test Theme".to_string()]);

        // Load custom theme
        let loaded = load_custom_theme(&safe_name);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.metadata.name, "Test Theme");
        assert_eq!(loaded.colors.panel_bg, "#111111");

        // Delete theme
        let delete_res = delete_custom_theme(&safe_name);
        assert!(delete_res.is_ok());
        assert!(!expected_dir.exists());

        let post_delete_themes = list_custom_themes();
        assert!(post_delete_themes.is_empty());
    }

    #[test]
    fn test_import_theme_invalid_json() {
        let _guard = TempDirGuard::new("invalid_json");

        let invalid_json = "{ invalid }";
        let res = import_theme_from_string(invalid_json);
        assert!(res.is_err());
    }

    #[test]
    fn test_import_theme_json_file() {
        let guard = TempDirGuard::new("import_file");

        let json_theme = r##"{
            "metadata": {
                "name": "File Theme",
                "author": "Author",
                "version": "1.0.0",
                "update_url": null
            },
            "colors": {
                "dark_mode": true,
                "panel_bg": "#111111",
                "window_bg": "#222222",
                "text_color": "#333333",
                "strong_text_color": "#444444",
                "accent_color": "#555555",
                "border_color": "#666666",
                "widget_bg": "#777777",
                "widget_hover": "#888888",
                "widget_active": "#999999",
                "success_color": null,
                "warning_color": null,
                "error_color": null,
                "info_color": null,
                "playfield_color": null,
                "playfield_opacity": null
            },
            "spacing": null
        }"##;

        let source_path = guard.path.join("external_theme.json");
        fs::write(&source_path, json_theme).unwrap();

        let import_res = import_theme_json(&source_path);
        assert!(import_res.is_ok());
        assert_eq!(import_res.unwrap(), "File Theme");

        assert!(load_custom_theme("File Theme").is_some());
    }
}
