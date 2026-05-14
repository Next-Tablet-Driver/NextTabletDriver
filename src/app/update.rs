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
        if ctx.input(|i| i.viewport().close_requested()) && !self.force_close {
            log::info!(target: "App", "Shutdown requested via window close");
            self.shared.shutdown_requested.store(true, Ordering::SeqCst);
        }

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
        self.sync_config(ctx, config, initial_config);

        // 7. Repaint Strategy
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

impl TabletMapperApp {
    fn sync_config(
        &mut self,
        ctx: &egui::Context,
        config: crate::core::config::models::MappingConfig,
        initial: crate::core::config::models::MappingConfig,
    ) {
        if config != initial {
            let is_interacting = ctx.input(|i| i.pointer.any_down());
            if !is_interacting && self.metrics.last_hz_update.elapsed() > Duration::from_millis(1000) {
                log::info!(target: "Config", "Configuration changed via UI");
            }
            if config.theme != initial.theme {
                crate::ui::theme::apply_theme(ctx, config.theme);
            }
            {
                let mut shared_config = self.shared.config.write().unwrap_or_log("config");
                *shared_config = config.clone();
                self.shared.config_version.fetch_add(1, Ordering::SeqCst);
            }
            let _ = self.save_sender.try_send(config);
        }
    }
}
