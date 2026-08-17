//! # Overlay Rendering
//!
//! Renders everything drawn on top of the main tab layout: confirmation and
//! warning modals, toast notifications, and the detachable debugger and
//! performance viewports.

use crate::app::state::{TabletMapperApp, ToastLevel, UiSnapshot};
use crate::t;
use eframe::egui::{self, Shadow};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Duration before a toast notification auto-dismisses.
const TOAST_DURATION: Duration = Duration::from_secs(3);

impl TabletMapperApp {
    /// Renders all non-main window elements (modals, toasts, viewports).
    pub fn render_overlays(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
        crate::ui::components::update_dialog::render_update_dialog(self, ctx);
        self.render_close_confirmation(ctx);
        self.render_toasts(ctx);
        self.render_debugger_window(ctx, snapshot);
        self.render_performance_window(ctx, snapshot);
        self.render_udev_warning(ctx);
    }

    pub fn render_close_confirmation(&mut self, ctx: &egui::Context) {
        if !self.show_close_confirm {
            return;
        }

        let frame = egui::Frame::window(&ctx.style()).shadow(Shadow::NONE);
        egui::Window::new(t!("dialog.unsaved.title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(frame)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(t!("dialog.unsaved.message"));
                    ui.label(t!("dialog.unsaved.detail"));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button(t!("dialog.unsaved.cancel")).clicked() {
                            self.show_close_confirm = false;
                        }
                        ui.add_space(8.0);
                        if ui
                            .button(
                                egui::RichText::new(t!("dialog.unsaved.close"))
                                    .color(crate::ui::theme::semantic_colors(ctx).error),
                            )
                            .clicked()
                        {
                            if let Err(e) = crate::settings::save_last_session(&self.profile.last_saved) {
                                log::error!(target: "App", "Failed to save last session on exit: {e}");
                            }
                            self.force_close = true;
                            self.show_close_confirm = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(4.0);
                });
            });
    }

    pub fn render_udev_warning(&mut self, ctx: &egui::Context) {
        if !self.missing_udev_rules {
            return;
        }

        let frame = egui::Frame::window(&ctx.style()).shadow(Shadow::NONE);
        egui::Window::new(t!("dialog.udev.title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(frame)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(t!("dialog.udev.message")).strong());
                    ui.add_space(8.0);
                    ui.label(t!("dialog.udev.detail_1"));
                    ui.label(t!("dialog.udev.detail_2"));
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new(t!("dialog.udev.how_to_fix")).strong());
                    ui.add_space(4.0);

                    let code_bg = ctx.style().visuals.faint_bg_color;
                    egui::Frame::new()
                        .fill(code_bg)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("sudo cp scripts/99-nexttabletdriver.rules /etc/udev/rules.d/").monospace());
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("sudo udevadm control --reload-rules && sudo udevadm trigger").monospace());
                            });
                        });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(t!("dialog.udev.no_relog")).italics());
                    ui.add_space(12.0);
                    if ui.button(t!("dialog.udev.done")).clicked() {
                        self.missing_udev_rules = false;
                    }
                    ui.add_space(4.0);
                });
            });
    }

    pub fn render_toasts(&mut self, ctx: &egui::Context) {
        self.toasts
            .retain(|t| t.created_at.elapsed() < TOAST_DURATION);

        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        let semantic = crate::ui::theme::semantic_colors(ctx);
        let toast_text = ctx.style().visuals.strong_text_color();

        for (i, toast) in self.toasts.iter().enumerate() {
            let offset_y = (i as f32).mul_add(50.0, 10.0);
            let id = egui::Id::new("toast").with(i);

            let bg_color = match toast.level {
                ToastLevel::Info => semantic.success,
                ToastLevel::Warning => semantic.warning,
                ToastLevel::Error => semantic.error,
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
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&toast.message).color(toast_text),
                                )
                                .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        });
                });
        }
    }

    pub fn render_debugger_window(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
        if !self.show_debugger {
            return;
        }

        let mut close_requested = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("debugger_viewport"),
            egui::ViewportBuilder::default()
                .with_title(t!("debugger.title"))
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

    pub fn render_performance_window(&mut self, ctx: &egui::Context, snapshot: &UiSnapshot) {
        if !self.show_latency_stats {
            return;
        }

        let mut close_requested = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("performance_viewport"),
            egui::ViewportBuilder::default()
                .with_title(t!("performance.viewport_title"))
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
                            egui::RichText::new(t!("performance.title"))
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
                        &self.shared,
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
}
