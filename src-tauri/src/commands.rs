use next_tablet_driver::engine::state::{LockRecoveryExt, SharedState};
use std::sync::Arc;
use tauri::State;

#[derive(serde::Serialize)]
pub struct TabletStatus {
    pub connected: bool,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[tauri::command]
pub fn get_tablet_status(state: State<'_, Arc<SharedState>>) -> TabletStatus {
    let device = state.device.read().unwrap_or_log("device_read_tauri");
    TabletStatus {
        connected: device.vid != 0,
        name: device.name.clone(),
        width: device.physical_size.0,
        height: device.physical_size.1,
        max_x: device.hardware_size.0,
        max_y: device.hardware_size.1,
    }
}

#[derive(serde::Serialize)]
pub struct EngineMetrics {
    pub hz: f32,
    pub last_packet_count: u32,
}

#[tauri::command]
pub fn get_metrics(state: State<'_, Arc<SharedState>>) -> EngineMetrics {
    let _stats = state
        .pipeline
        .stats
        .read()
        .unwrap_or_log("stats_read_tauri");
    let count = state
        .pipeline
        .packet_count
        .load(std::sync::atomic::Ordering::Relaxed);
    EngineMetrics {
        hz: 0.0,
        last_packet_count: count,
    }
}

use next_tablet_driver::core::config::models::MappingConfig;
use next_tablet_driver::engine::state::WriteRecoverExt;

#[tauri::command]
pub fn get_config(state: State<'_, Arc<SharedState>>) -> MappingConfig {
    state
        .config
        .mapping
        .read()
        .unwrap_or_log("config_read_tauri")
        .clone()
}

#[tauri::command]
pub fn set_config(
    state: State<'_, Arc<SharedState>>,
    new_config: MappingConfig,
) -> Result<(), String> {
    let _ = next_tablet_driver::startup::set_run_at_startup(new_config.run_at_startup);
    next_tablet_driver::startup::set_fast_timer(new_config.force_high_resolution_timer);

    *state
        .config
        .mapping
        .write()
        .unwrap_or_reset("config_write_tauri") = new_config.clone();
    state
        .config
        .version
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn save_config(
    state: State<'_, Arc<SharedState>>,
    save_sender: State<'_, crossbeam_channel::Sender<MappingConfig>>,
) -> Result<(), String> {
    let current = state
        .config
        .mapping
        .read()
        .unwrap_or_log("config_read_tauri")
        .clone();
    let _ = save_sender.try_send(current);
    Ok(())
}

#[tauri::command]
pub fn reset_config(state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    *state
        .config
        .mapping
        .write()
        .unwrap_or_reset("config_write_tauri") = MappingConfig::default();
    state
        .config
        .version
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn load_profile(state: State<'_, Arc<SharedState>>, path: String) -> Result<(), String> {
    match next_tablet_driver::settings::load_settings_from_file(std::path::Path::new(&path)) {
        Ok((cfg, _)) => {
            let _ = next_tablet_driver::startup::set_run_at_startup(cfg.run_at_startup);
            next_tablet_driver::startup::set_fast_timer(cfg.force_high_resolution_timer);

            *state
                .config
                .mapping
                .write()
                .unwrap_or_reset("config_write_tauri") = cfg;
            state
                .config
                .version
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn import_otd_profile(state: State<'_, Arc<SharedState>>, path: String) -> Result<(), String> {
    match next_tablet_driver::settings::otd_import::import_otd_profile(std::path::Path::new(&path))
    {
        Ok(cfg) => {
            let _ = next_tablet_driver::startup::set_run_at_startup(cfg.run_at_startup);
            next_tablet_driver::startup::set_fast_timer(cfg.force_high_resolution_timer);

            *state
                .config
                .mapping
                .write()
                .unwrap_or_reset("config_write_tauri") = cfg;
            state
                .config
                .version
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn export_profile(state: State<'_, Arc<SharedState>>, path: String) -> Result<(), String> {
    let current = state
        .config
        .mapping
        .read()
        .unwrap_or_log("config_read_tauri")
        .clone();
    next_tablet_driver::settings::save_to_path(std::path::Path::new(&path), &current)
}

#[derive(serde::Serialize)]
pub struct PresetInfo {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub fn get_presets() -> Vec<PresetInfo> {
    next_tablet_driver::settings::list_profiles()
        .into_iter()
        .map(|(name, path)| PresetInfo {
            name,
            path: path.to_string_lossy().into_owned(),
        })
        .collect()
}

#[tauri::command]
pub fn get_current_profile_name() -> String {
    next_tablet_driver::settings::load_session_meta()
        .map(|meta| meta.profile_name)
        .unwrap_or_else(|| "Default Profile".to_string())
}

#[tauri::command]
pub fn get_logs() -> Vec<next_tablet_driver::logger::LogEntry> {
    next_tablet_driver::logger::LOG_BUFFER
        .read()
        .map(|b| b.iter().cloned().collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn clear_logs() {
    if let Ok(mut buffer) = next_tablet_driver::logger::LOG_BUFFER.write() {
        buffer.clear();
        next_tablet_driver::logger::LOG_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tauri::command]
pub fn get_releases()
-> Result<Vec<next_tablet_driver::application::services::autoupdate::models::Release>, String> {
    next_tablet_driver::application::services::autoupdate::github::fetch_releases()
}

#[tauri::command]
pub fn get_available_plugins(
    state: State<'_, Arc<SharedState>>,
) -> Vec<ntd_plugin_api::PluginManifest> {
    let manifests = state.plugins.loaded_manifests();
    log::info!(target: "Plugins", "Frontend requested available plugins. Found {} plugins.", manifests.len());
    manifests
}

#[tauri::command]
pub fn reload_plugins(state: State<'_, Arc<SharedState>>) -> Vec<ntd_plugin_api::PluginManifest> {
    state.plugins.reload();
    state.plugins.loaded_manifests()
}

#[tauri::command]
pub fn open_plugins_folder(state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    let dir = state.plugins.primary_plugins_dir();
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Maximum size accepted for a plugin library (guards against accidental/huge files).
const MAX_PLUGIN_SIZE: u64 = 64 * 1024 * 1024;

/// Lets the user pick a plugin library through a native dialog, asks for an explicit
/// confirmation (plugins run native code with the app's privileges) and installs it.
///
/// The file selection happens in the backend on purpose: the webview is never trusted to
/// provide a filesystem path, so a compromised frontend cannot plant a library in the
/// plugins directory. Returns `Ok(false)` if the user cancelled.
#[tauri::command]
pub async fn install_plugin(
    app: tauri::AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<bool, String> {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let ext = if cfg!(windows) { "dll" } else { "so" };
    let Some(picked) = app
        .dialog()
        .file()
        .add_filter("Plugin Library", &[ext])
        .blocking_pick_file()
    else {
        return Ok(false);
    };
    let source = picked.into_path().map_err(|e| e.to_string())?;

    if source.extension().and_then(|e| e.to_str()) != Some(ext) {
        return Err(format!("Only .{ext} plugin libraries can be installed"));
    }
    let meta = std::fs::metadata(&source).map_err(|e| format!("Cannot read plugin file: {e}"))?;
    if !meta.is_file() || meta.len() > MAX_PLUGIN_SIZE {
        return Err("Invalid plugin file".to_string());
    }
    let file_name = source.file_name().ok_or("Invalid file name")?.to_owned();

    let confirmed = app
        .dialog()
        .message(format!(
            "Plugins run native code with full access to your system.
Only install plugins from authors you trust.

Install \"{}\"?",
            file_name.to_string_lossy()
        ))
        .title("Install plugin")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Install".into(),
            "Cancel".into(),
        ))
        .blocking_show();
    if !confirmed {
        return Ok(false);
    }

    let dir = state.plugins.primary_plugins_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create plugins dir: {e}"))?;
    let dest = dir.join(&file_name);
    std::fs::copy(&source, &dest).map_err(|e| format!("Failed to copy plugin: {e}"))?;

    // The user has just confirmed this exact file: remember its hash so it is allowed to load.
    state.plugins.trust_installed_file(&dest)?;
    state.plugins.reload();
    Ok(true)
}

/// Plugin libraries found in the plugins folder that are not trusted and were not loaded.
#[tauri::command]
pub fn get_untrusted_plugins(
    state: State<'_, Arc<SharedState>>,
) -> Vec<next_tablet_driver::engine::plugins::UntrustedPlugin> {
    state.plugins.untrusted_plugins()
}

/// Asks the user (natively, not through the webview) whether to trust a plugin library found
/// in the plugins folder, then approves exactly that content hash. Returns `Ok(false)` if the
/// user declined.
#[tauri::command]
pub async fn trust_plugin(
    app: tauri::AppHandle,
    state: State<'_, Arc<SharedState>>,
    sha256: String,
) -> Result<bool, String> {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let pending = state
        .plugins
        .untrusted_plugins()
        .into_iter()
        .find(|p| p.sha256 == sha256)
        .ok_or("This plugin is no longer pending approval")?;

    let short_hash: String = pending.sha256.chars().take(16).collect();
    let confirmed = app
        .dialog()
        .message(format!(
            "\"{}\" was found in the plugins folder but was not installed through NextTabletDriver.\n\nPlugins run native code with full access to your system. Only trust it if you know where it comes from.\n\nSHA-256: {short_hash}...",
            pending.file_name
        ))
        .title("Trust this plugin?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Trust and load".into(), "Cancel".into()))
        .blocking_show();
    if !confirmed {
        return Ok(false);
    }

    state.plugins.trust_plugin(&sha256)?;
    Ok(true)
}

#[tauri::command]
pub fn delete_plugin(state: State<'_, Arc<SharedState>>, id: String) -> Result<(), String> {
    state.plugins.delete_plugin(&id).map(|_| ())
}

/// Theme files found in the themes folder; the frontend parses and validates them.
#[tauri::command]
pub fn list_themes() -> Vec<next_tablet_driver::settings::themes::ThemeFile> {
    next_tablet_driver::settings::themes::list_theme_files()
}

/// Lets the user pick a theme file with a native dialog and copies it into the themes folder.
/// The webview never supplies a path. Returns `Ok(None)` if the dialog was cancelled.
#[tauri::command]
pub async fn import_theme(
    app: tauri::AppHandle,
) -> Result<Option<next_tablet_driver::settings::themes::ThemeFile>, String> {
    use tauri_plugin_dialog::DialogExt;

    let Some(picked) = app
        .dialog()
        .file()
        .add_filter("Theme", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    next_tablet_driver::settings::themes::import_theme_file(&path).map(Some)
}

#[tauri::command]
pub fn delete_theme(id: String) -> Result<(), String> {
    next_tablet_driver::settings::themes::delete_theme_file(&id)
}

#[tauri::command]
pub fn open_themes_folder() -> Result<(), String> {
    let dir = next_tablet_driver::settings::themes::themes_dir();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
