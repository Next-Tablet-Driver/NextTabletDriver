//! # Application Update Loop
//!
//! This module contains the implementation of the `eframe::App` trait for
//! `TabletMapperApp`.

use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::engine::state::LockRecoveryExt;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

impl eframe::App for TabletMapperApp {
    /// The main application loop called by egui.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 0. Handle graceful shutdown signal
        if ctx.input(|i| i.viewport().close_requested())
            && !self.force_close
            && self.shared.is_visible.load(Ordering::Acquire)
        {
            log::info!(target: "App", "Shutdown requested via window close");
            self.shared.shutdown_requested.store(true, Ordering::SeqCst);
        }

        if !self.shared.is_visible.load(Ordering::Acquire) {
            // Drain the tablet channel to prevent unbounded memory growth.
            // The engine thread already avoids sending when invisible (see
            // `tablet_manager.rs`), but we drain defensively in case of a
            // race during the visibility transition.
            while self.tablet_receiver.try_recv().is_ok() {}

            // Drain update channel so we don't miss update notifications
            if let Ok(status) = self.update_receiver.try_recv() {
                self.update_status = status;
            }

            return;
        }

        // ── Normal Rendering Path ─────────────────────────────────────────

        self.check_theme_download(ctx);

        // 1. Capture snapshot for the entire frame
        let snapshot = UiSnapshot::capture(&self.shared);

        // 2. Process Input/IO Events
        self.process_io_events(ctx, &snapshot);

        // 3. Handle Lifecycle (tray, close guard, etc)
        self.handle_lifecycle(ctx, &snapshot.config);

        // 4. Render Layout & Panels
        let mut config = snapshot.config.clone();
        let initial_config = config.clone();

        self.render_main_layout(ctx, &mut config, &snapshot);

        // 5. Render Overlays (Dialogs, Toasts, Viewports)
        self.render_overlays(ctx, &snapshot);

        // 6. State Persistence (Sync config back if changed)
        self.sync_config(ctx, config, &initial_config);

        // 7. Repaint Strategy
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

impl TabletMapperApp {
    fn check_theme_download(&mut self, ctx: &egui::Context) {
        let got_result = if let Ok(mut guard) = self.theme_download_result.try_lock()
            && guard.is_some()
        {
            guard.take()
        } else {
            None
        };

        if let Some(result) = got_result {
            let theme_name = self.theme_downloading_name.take().unwrap_or_default();
            match result {
                Ok(safe_name) => {
                    self.app_prefs.theme =
                        crate::core::config::models::ThemePreference::Custom(safe_name.clone());
                    crate::ui::theme::apply_theme(ctx, &self.app_prefs.theme);
                    crate::settings::app_preferences::save_app_preferences(&self.app_prefs);
                    log::info!(
                        target: "ThemeStore",
                        "{} ({theme_name})",
                        crate::t!("settings.theme.download_success")
                    );
                    crate::app::telemetry::capture_event(
                        "theme_downloaded",
                        Some(serde_json::json!({ "theme_name": safe_name })),
                        &self.app_prefs,
                    );
                }
                Err(e) => {
                    log::error!(
                        target: "ThemeStore",
                        "{} {e}",
                        crate::t!("settings.theme.download_error")
                    );
                }
            }
        }
    }

    fn sync_config(
        &self,
        ctx: &egui::Context,
        config: crate::core::config::models::MappingConfig,
        initial: &crate::core::config::models::MappingConfig,
    ) {
        if config != *initial {
            let is_interacting = ctx.input(|i| i.pointer.any_down());
            if !is_interacting && self.metrics.last_hz_update.elapsed() > Duration::from_secs(1) {
                log::info!(target: "Config", "Configuration changed via UI");
            }
            {
                let mut shared_config = self.shared.config.write().unwrap_or_log("config");
                *shared_config = config.clone();
                self.shared.config_version.fetch_add(1, Ordering::SeqCst);
                drop(shared_config);
            }
            let _ = self.save_sender.try_send(config);
        }
    }
}
