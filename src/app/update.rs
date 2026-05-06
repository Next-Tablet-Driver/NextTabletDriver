//! # Application Update Loop
//!
//! This module contains the implementation of the `eframe::App` trait for
//! `TabletMapperApp`. The `update` function defined here is called by egui
//! every frame to process events, update state, and render the user interface.

use crate::app::state::{AppTab, TabletMapperApp, ToastLevel, UiSnapshot};
use crate::engine::state::LockResultExt;
use crate::ui::panels::console::render_console_panel;

use crate::ui::panels::filters::render_filters_panel;
use crate::ui::panels::output::render_output_panel;
use crate::ui::panels::pen_settings::render_pen_settings_panel;
use crate::ui::panels::release::render_release_panel;
use crate::ui::panels::settings::render_settings_panel;
use eframe::egui::{self, Shadow};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Duration before a toast notification auto-dismisses.
const TOAST_DURATION: Duration = Duration::from_secs(3);

impl eframe::App for TabletMapperApp {
    /// The main application loop called by egui.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
    /// Processes pending hardware events and background thread messages.
    fn process_io_events(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
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

            let mut shared_data = self.shared.tablet_data.write().ignore_poison();
            *shared_data = data;

            let needs_live_update = self.show_debugger
                || self.show_latency_stats
                || self.active_tab == AppTab::Console;

            if needs_live_update {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        }

        // Check for updates
        if let Ok(status) = self.update_receiver.try_recv() {
            if let crate::app::autoupdate::UpdateStatus::Available(release) = &status {
                log::info!(target: "Update", "Update available: {}", release.tag_name);
            }
            self.update_status = status;
        }
    }

    /// Handles application-level lifecycle events like minimization and closing.
    fn handle_lifecycle(
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
                log::info!(target: "Tray", "Window minimized, hiding to system tray...");
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            self.was_minimized = is_minimized;
        }
    }

    /// Renders the primary application interface.
    fn render_main_layout(
        &mut self,
        ctx: &egui::Context,
        config: &mut crate::core::config::models::MappingConfig,
        snapshot: &UiSnapshot,
    ) {
        let (min_x, min_y, max_x, max_y) = self.calculate_display_bounds();

        crate::ui::components::menu_bar::render_menu_bar(self, ctx, snapshot);
        crate::ui::components::tabs::render_tabs(self, ctx);
        crate::ui::components::footer::render_footer(self, ctx, config, snapshot);

        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            AppTab::Output => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_output_panel(self, ui, config, snapshot, min_x, min_y, max_x, max_y);
                });
            }
            AppTab::Filters => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_filters_panel(self, ui, config, snapshot);
                });
            }
            AppTab::PenSettings => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_pen_settings_panel(self, ui, config, snapshot);
                });
            }
            AppTab::Console => render_console_panel(self, ui),
            AppTab::Settings => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_settings_panel(self, ui, config, snapshot);
                });
            }
            AppTab::Release => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_release_panel(self, ui);
                });
            }

        });
    }

    /// Renders all non-main window elements (modals, toasts, viewports).
    fn render_overlays(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
        crate::ui::components::update_dialog::render_update_dialog(self, ctx);
        self.render_close_confirmation(ctx);
        self.render_toasts(ctx);
        self.render_debugger_window(ctx, snapshot);
        self.render_performance_window(ctx, snapshot);
    }

    fn render_close_confirmation(&mut self, ctx: &egui::Context) {
        if !self.show_close_confirm {
            return;
        }

        let frame = egui::Frame::window(&ctx.style()).shadow(Shadow::NONE);
        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(frame)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label("Are you sure you want to close the application?");
                    ui.label("The current profile has unsaved changes.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_close_confirm = false;
                        }
                        ui.add_space(8.0);
                        if ui
                            .button(
                                egui::RichText::new("Close Anyway")
                                    .color(egui::Color32::from_rgb(220, 80, 80)),
                            )
                            .clicked()
                        {
                            let _ = crate::settings::save_last_session(&self.profile.last_saved);
                            self.force_close = true;
                            self.show_close_confirm = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(4.0);
                });
            });
    }

    fn render_toasts(&mut self, ctx: &egui::Context) {
        self.toasts
            .retain(|t| t.created_at.elapsed() < TOAST_DURATION);

        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        for (i, toast) in self.toasts.iter().enumerate() {
            let offset_y = i as f32 * 50.0 + 10.0;
            let id = egui::Id::new("toast").with(i);

            let (bg_color, text_color) = match toast.level {
                ToastLevel::Info => (egui::Color32::from_rgb(40, 120, 60), egui::Color32::WHITE),
                ToastLevel::Warning => {
                    (egui::Color32::from_rgb(180, 130, 30), egui::Color32::WHITE)
                }
                ToastLevel::Error => (egui::Color32::from_rgb(180, 50, 50), egui::Color32::WHITE),
            };

            egui::Area::new(id)
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, offset_y))
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(bg_color)
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(&toast.message).color(text_color));
                        });
                });
        }
    }

    fn render_debugger_window(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
        if !self.show_debugger {
            return;
        }

        let mut close_requested = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("debugger_viewport"),
            egui::ViewportBuilder::default()
                .with_title("Tablet Debugger")
                .with_inner_size([600.0, 750.0])
                .with_resizable(true),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    close_requested = true;
                }

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.metrics
                        .update_hz(self.shared.packet_count.load(Ordering::Relaxed));

                    ui.vertical_centered(|ui| {
                        ui.add_space(5.0);
                        ui.heading(
                            egui::RichText::new(&snapshot.tablet_name)
                                .strong()
                                .extra_letter_spacing(1.5),
                        );
                    });

                    crate::ui::panels::debugger::render_debugger_panel(
                        snapshot,
                        self.metrics.displayed_hz,
                        ui,
                    );
                });
            },
        );

        if close_requested {
            self.show_debugger = false;
        }
    }

    fn render_performance_window(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
        if !self.show_latency_stats {
            return;
        }

        let mut close_requested = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("performance_viewport"),
            egui::ViewportBuilder::default()
                .with_title("Input Lag & Performance Analysis")
                .with_inner_size([500.0, 600.0])
                .with_resizable(true),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    close_requested = true;
                }

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.metrics
                        .update_hz(self.shared.packet_count.load(Ordering::Relaxed));

                    ui.vertical_centered(|ui| {
                        ui.add_space(5.0);
                        ui.heading(
                            egui::RichText::new("Driver Performance Monitor")
                                .strong()
                                .extra_letter_spacing(1.0),
                        );
                    });

                    if crate::ui::panels::performance::render_performance_panel(
                        snapshot,
                        self.metrics.displayed_hz,
                        self.metrics.ui_latency_ms,
                        self.metrics.min_ui_latency_ms,
                        self.metrics.max_ui_latency_ms,
                        self.metrics.avg_ui_latency_ms,
                        ui,
                        self.shared.clone(),
                    ) {
                        self.metrics.reset_ui_latency();
                    }
                });
            },
        );

        if close_requested {
            self.show_latency_stats = false;
        }
    }

    fn sync_config(
        &mut self,
        ctx: &egui::Context,
        config: crate::core::config::models::MappingConfig,
        initial: crate::core::config::models::MappingConfig,
    ) {
        if config != initial {
            let is_interacting = ctx.input(|i| i.pointer.any_down());
            if !is_interacting && self.last_config_log.elapsed() > Duration::from_millis(1000) {
                log::info!(target: "Config", "Configuration changed via UI");
                self.last_config_log = Instant::now();
            }
            if config.theme != initial.theme {
                crate::ui::theme::apply_theme(ctx, config.theme);
            }
            {
                let mut shared_config = self.shared.config.write().ignore_poison();
                *shared_config = config.clone();
                self.shared.config_version.fetch_add(1, Ordering::SeqCst);
            }
            if self.save_sender.try_send(config).is_err() {
                log::warn!(target: "Config", "Save queue full, config change dropped");
            }
        }
    }

    fn calculate_display_bounds(&self) -> (f32, f32, f32, f32) {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (0.0, 0.0, 1920.0, 1080.0);
        if !self.displays.is_empty() {
            let mut mx = i32::MAX;
            let mut my = i32::MAX;
            let mut ax = i32::MIN;
            let mut ay = i32::MIN;
            for d in &self.displays {
                mx = mx.min(d.x);
                my = my.min(d.y);
                ax = ax.max(d.x + d.width as i32);
                ay = ay.max(d.y + d.height as i32);
            }
            min_x = mx as f32;
            min_y = my as f32;
            max_x = ax as f32;
            max_y = ay as f32;
        }
        (min_x, min_y, max_x, max_y)
    }
}
