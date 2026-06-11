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
    fs::read_to_string(path).map_or(None, |content| serde_json::from_str(&content).ok())
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
    let config: ThemeConfig =
        serde_json::from_str(content).map_err(|e| format!("Invalid theme format: {e}"))?;

    let safe_name = crate::settings::sanitize_profile_name(&config.metadata.name);
    let theme_folder = get_themes_dir().join(&safe_name);

    if !theme_folder.exists() {
        fs::create_dir_all(&theme_folder).map_err(|e| e.to_string())?;
    }

    fs::write(theme_folder.join("theme.json"), content).map_err(|e| e.to_string())?;

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
    }
    Ok(())
}
