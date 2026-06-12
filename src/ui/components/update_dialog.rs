use crate::app::state::TabletMapperApp;
use crate::t;
use eframe::egui;

pub fn render_update_dialog(app: &mut TabletMapperApp, ctx: &egui::Context) {
    if let crate::app::autoupdate::UpdateStatus::Available(release) = &app.update_status {
        let screen_rect = ctx.content_rect();

        // Semi-transparent backdrop to dim the content behind the dialog.
        // Uses a dark overlay regardless of theme - this is intentional for modal focus.
        egui::Area::new(egui::Id::new("update_overlay"))
            .interactable(true)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(180));
            });

        let mut open = true;
        let version = release.tag_name.clone();
        let body = release
            .body
            .clone()
            .unwrap_or_else(|| t!("dialog.update.no_changelog"));

        let v = ctx.style().visuals.clone();
        let dialog_bg = v.window_fill;
        let header_bg = v.panel_fill;
        let border_color = v.widgets.noninteractive.bg_stroke.color;
        let strong_text = v.strong_text_color();
        let weak_text = v.weak_text_color();
        let text_color = v.text_color();
        let accent = v.selection.bg_fill;
        // Use a contrasting text on the accent button: white for dark accents, strong_text for light ones
        let accent_text = strong_text;

        egui::Window::new(t!("dialog.update.title"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center())
            .fixed_size([450.0, 350.0])
            .open(&mut open)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(dialog_bg)
                    .corner_radius(4.0)
                    .inner_margin(0.0)
                    .stroke(egui::Stroke::new(1.0, border_color)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    egui::Frame::new()
                        .fill(header_bg)
                        .corner_radius(egui::CornerRadius {
                            nw: 12,
                            ne: 12,
                            sw: 0,
                            se: 0,
                        })
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new(t!("dialog.update.header"))
                                        .size(24.0)
                                        .strong()
                                        .color(strong_text),
                                );
                                ui.add_space(5.0);
                                ui.label(
                                    egui::RichText::new(t!(
                                        "dialog.update.version",
                                        version = &version
                                    ))
                                    .size(14.0)
                                    .color(weak_text),
                                );
                            });
                        });

                    ui.add_space(15.0);

                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(20, 0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(t!("dialog.update.whats_new"))
                                    .strong()
                                    .size(16.0)
                                    .color(strong_text),
                            );
                            ui.add_space(8.0);

                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(body).size(13.0).color(text_color),
                                    );
                                });
                        });

                    ui.add_space(30.0);

                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(20, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let later_btn = egui::Button::new(
                                    egui::RichText::new(t!("dialog.update.later"))
                                        .size(14.0)
                                        .color(text_color),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, border_color))
                                .min_size(egui::vec2(120.0, 36.0));

                                if ui.add(later_btn).clicked() {
                                    app.dismiss_update();
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let update_btn = egui::Button::new(
                                            egui::RichText::new(t!("dialog.update.install"))
                                                .size(14.0)
                                                .strong()
                                                .color(accent_text),
                                        )
                                        .fill(accent)
                                        .corner_radius(4.0)
                                        .min_size(egui::vec2(160.0, 36.0));

                                        if ui.add(update_btn).clicked() {
                                            app.start_update();
                                        }
                                    },
                                );
                            });
                        });

                    ui.add_space(5.0);
                });
            });
    }
}
