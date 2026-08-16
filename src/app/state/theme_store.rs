//! State and background fetch/download logic for the online theme store.

use super::TabletMapperApp;
use crate::core::config::theme_models::ThemeMetadata;

#[derive(Clone, Debug)]
pub struct ThemeStoreItem {
    pub metadata: ThemeMetadata,
    pub dark_mode: bool,
}

pub type ThemeStoreResult = Result<Vec<ThemeStoreItem>, String>;

/// UI state for the online theme store viewport and background theme downloads.
pub struct ThemeStoreState {
    /// Toggle to render the theme store viewport.
    pub open: bool,
    /// True while the remote theme list is being fetched.
    pub loading: bool,
    /// Cached result of the last theme store listing request.
    pub list: std::sync::Arc<std::sync::Mutex<Option<ThemeStoreResult>>>,
    /// Sub-string filter for searching the theme store.
    pub search: String,
    /// Optional dark/light filter for the theme store (`None` means "all").
    pub filter_mode: Option<bool>,
    /// Result of the last background theme download, if any.
    pub download_result: std::sync::Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    /// Name of the theme currently being downloaded, if any.
    pub downloading_name: Option<String>,
}

impl TabletMapperApp {
    pub fn fetch_theme_store_list(&mut self) {
        let is_none = self.theme_store.list.lock().map_or(true, |g| g.is_none());
        if is_none {
            self.theme_store.loading = true;
            let list_arc = std::sync::Arc::clone(&self.theme_store.list);
            std::thread::spawn(move || {
                let result = crate::settings::themes::fetch_theme_store_list_sync();
                if let Ok(mut guard) = list_arc.lock() {
                    *guard = Some(result);
                }
            });
        }
    }

    pub fn download_theme(&mut self, theme: &str, ctx: &eframe::egui::Context) {
        if self.theme_store.downloading_name.is_some() {
            return; // Only allow one download at a time
        }

        let theme_name = theme.to_string();
        self.theme_store.downloading_name = Some(theme_name.clone());

        let result_arc = std::sync::Arc::clone(&self.theme_store.download_result);
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let result = crate::settings::themes::download_and_install_theme_sync(&theme_name);
            if let Ok(mut guard) = result_arc.lock() {
                *guard = Some(result);
            }
            // Request UI repaint immediately after download completes
            ctx_clone.request_repaint();
        });
    }
}
