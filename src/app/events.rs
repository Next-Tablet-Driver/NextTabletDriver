use crate::app::state::{AppTab, TabletMapperApp, UiSnapshot};
use crate::engine::state::WriteRecoverExt;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

impl TabletMapperApp {
    /// Processes pending hardware events and background thread messages.
    pub fn process_io_events(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
        // Keyboard Shortcuts
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::S,
            ))
        }) {
            self.save_settings(&snapshot.config);
        }

        // Drain pending tablet events, keeping only the latest
        let mut last_data = None;
        while let Ok(data) = self.tablet_receiver.try_recv() {
            last_data = Some(data);
        }

        if let Some(data) = last_data {
            if let Some(receive_time) = data.receive_time {
                self.metrics
                    .update_latency(receive_time.elapsed().as_secs_f32() * 1000.0);
            }

            {
                let mut shared_data = self
                    .shared
                    .pipeline
                    .tablet_data
                    .write()
                    .unwrap_or_reset("tablet_data");
                *shared_data = data;
            }

            let needs_live_update =
                self.show_debugger || self.show_latency_stats || self.active_tab == AppTab::Console;

            if needs_live_update {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        }

        // Check for updates
        let mut got_update = false;
        while let Ok(status) = self.update_receiver.try_recv() {
            if let crate::app::autoupdate::UpdateStatus::Available(release) = &status {
                log::info!(target: "Update", "Update available: {}", release.tag_name);
            }
            self.update_status = status;
            got_update = true;
        }
        if got_update {
            ctx.request_repaint();
        }
    }

    /// Handles application-level lifecycle events like minimization and closing.
    pub fn handle_lifecycle(
        &mut self,
        ctx: &egui::Context,
        config: &crate::core::config::models::MappingConfig,
    ) {
        // Close Guard
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        let is_dirty = self.profile.is_dirty(config);

        if close_requested && is_dirty && !self.show_close_confirm && !self.force_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.show_close_confirm = true;
        }

        // System Tray Minimize
        if config.system_tray_on_minimize {
            let is_minimized = ctx.input(|i| i.viewport().minimized).unwrap_or(false);
            if is_minimized && !self.was_minimized {
                log::info!(target: "Tray", "Window minimized, closing eframe to sit in system tray...");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                self.shared
                    .lifecycle
                    .is_visible
                    .store(false, Ordering::Release);
                self.force_close = true;
                ctx.request_repaint();
            }
            self.was_minimized = is_minimized;
        }
    }
}
