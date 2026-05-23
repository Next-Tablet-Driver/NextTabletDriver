use crate::app::state::TabletMapperApp;
use eframe::egui;

pub fn render_update_dialog(app: &mut TabletMapperApp, ctx: &egui::Context) {
    if let crate::app::autoupdate::UpdateStatus::Available(release) = &app.update_status {
        let screen_rect = ctx.content_rect();
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
            .unwrap_or_else(|| "No changelog provided.".to_string());

        egui::Window::new("Update Available")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center())
            .fixed_size([450.0, 350.0])
            .open(&mut open)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgb(20, 22, 25))
                    .corner_radius(4.0)
                    .inner_margin(0.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60))),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(30, 34, 40))
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
                                    egui::RichText::new("Update Available!")
                                        .size(24.0)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                );
                                ui.add_space(5.0);
                                ui.label(
                                    egui::RichText::new(format!("Version {version}"))
                                        .size(14.0)
                                        .color(egui::Color32::from_gray(180)),
                                );
                            });
                        });

                    ui.add_space(15.0);

                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(20, 0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("What's new:")
                                    .strong()
                                    .size(16.0)
                                    .color(egui::Color32::LIGHT_GRAY),
                            );
                            ui.add_space(8.0);

                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(body)
                                            .size(13.0)
                                            .color(egui::Color32::from_gray(160)),
                                    );
                                });
                        });

                    ui.add_space(30.0);

                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(20, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let later_btn = egui::Button::new(
                                    egui::RichText::new("Remind Me Later")
                                        .size(14.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, egui::Color32::WHITE))
                                .min_size(egui::vec2(120.0, 36.0));

                                if ui.add(later_btn).clicked() {
                                    app.dismiss_update();
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let update_btn = egui::Button::new(
                                            egui::RichText::new("Install Update")
                                                .size(14.0)
                                                .strong()
                                                .color(egui::Color32::BLACK),
                                        )
                                        .fill(egui::Color32::from_rgb(0, 120, 215))
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
