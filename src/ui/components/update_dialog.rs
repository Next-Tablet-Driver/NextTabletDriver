use crate::app::state::TabletMapperApp;
use crate::t;
use eframe::egui;

fn render_markdown(
    ui: &mut egui::Ui,
    text: &str,
    text_color: egui::Color32,
    strong_color: egui::Color32,
) {
    ui.spacing_mut().item_spacing.y = 4.0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            ui.add_space(6.0);
            continue;
        }

        if line.starts_with("##") || line.starts_with('#') {
            let clean = line.trim_start_matches('#').trim();
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(clean)
                        .strong()
                        .size(15.0)
                        .color(strong_color),
                )
                .wrap_mode(egui::TextWrapMode::Wrap),
            );
            ui.add_space(2.0);
        } else if line.starts_with("- ") || line.starts_with("* ") {
            let clean = line[2..].trim().replace("**", "");
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("•").strong().color(strong_color));
                ui.add(
                    egui::Label::new(egui::RichText::new(clean).size(13.0).color(text_color))
                        .wrap_mode(egui::TextWrapMode::Wrap),
                );
            });
        } else {
            let clean = line.replace("**", "");
            ui.add(
                egui::Label::new(egui::RichText::new(clean).size(13.0).color(text_color))
                    .wrap_mode(egui::TextWrapMode::Wrap),
            );
        }
    }
}

pub fn render_update_dialog(app: &mut TabletMapperApp, ctx: &egui::Context) {
    let (is_downloading, dl_stats, release) = match &app.update_status {
        crate::app::autoupdate::UpdateStatus::Available(release) => (false, None, Some(release)),
        crate::app::autoupdate::UpdateStatus::Downloading(p) => (true, Some(p.clone()), None),
        _ => return,
    };

    let screen_rect = ctx.content_rect();

    // Semi-transparent backdrop
    egui::Area::new(egui::Id::new("update_overlay"))
        .interactable(true)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(200));
        });

    let mut open = true;
    let version = release.map_or_else(|| "Updating...".to_string(), |r| r.tag_name.clone());
    let body = release
        .and_then(|r| r.body.clone())
        .unwrap_or_else(|| t!("dialog.update.no_changelog"));

    let v = ctx.style().visuals.clone();
    let dialog_bg = v.window_fill;
    let header_bg = v.panel_fill;
    let border_color = v.widgets.noninteractive.bg_stroke.color;
    let strong_text = v.strong_text_color();
    let weak_text = v.weak_text_color();
    let text_color = v.text_color();
    let accent = v.selection.bg_fill;
    let accent_text = strong_text;

    egui::Window::new(t!("dialog.update.title"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(screen_rect.center())
        .fixed_size([520.0, 420.0])
        .open(&mut open)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(dialog_bg)
                .corner_radius(12.0)
                .inner_margin(0.0)
                .stroke(egui::Stroke::new(1.0, border_color)),
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // HEADER AREA
                egui::Frame::new()
                    .fill(header_bg)
                    .corner_radius(egui::CornerRadius {
                        nw: 12,
                        ne: 12,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(egui::Margin::symmetric(24, 24))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            let icon = if is_downloading {
                                egui_phosphor::regular::CLOUD_ARROW_DOWN
                            } else {
                                egui_phosphor::regular::SPARKLE
                            };

                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {}",
                                    icon,
                                    t!("dialog.update.header")
                                ))
                                .size(28.0)
                                .strong()
                                .color(strong_text),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(if is_downloading {
                                    "Downloading update...".to_string()
                                } else {
                                    t!("dialog.update.version", version = &version)
                                })
                                .size(15.0)
                                .color(weak_text),
                            );
                        });
                    });

                ui.add_space(20.0);

                // BODY AREA
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(30, 0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::GIFT)
                                    .size(18.0)
                                    .color(accent),
                            );
                            ui.label(
                                egui::RichText::new(t!("dialog.update.whats_new"))
                                    .strong()
                                    .size(18.0)
                                    .color(strong_text),
                            );
                        });

                        ui.add_space(12.0);

                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                render_markdown(ui, &body, text_color, strong_text);
                            });
                    });

                ui.add_space(30.0);

                // FOOTER AREA
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(30, 15))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        if is_downloading {
                            if let Some(stats) = dl_stats {
                                ui.vertical_centered_justified(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.add_space(ui.available_width() / 2.0 - 55.0);
                                                ui.spinner();
                                                ui.add_space(8.0);
                                                ui.label(
                                                    egui::RichText::new("Updating...")
                                                        .strong()
                                                        .color(accent)
                                                        .size(15.0),
                                                );
                                            },
                                        );
                                    });

                                    ui.add_space(10.0);

                                    let mb_dl = stats.downloaded as f64 / 1_048_576.0;
                                    let mb_spd = stats.speed as f64 / 1_048_576.0;

                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{mb_dl:.1} MB ({mb_spd:.1} MB/s)"
                                        ))
                                        .color(weak_text)
                                        .size(14.0),
                                    );
                                });
                            }
                        } else {
                            ui.horizontal(|ui| {
                                let later_btn = egui::Button::new(
                                    egui::RichText::new(t!("dialog.update.later"))
                                        .size(15.0)
                                        .color(text_color),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, border_color))
                                .corner_radius(6.0)
                                .min_size(egui::vec2(130.0, 40.0));

                                if ui
                                    .add(later_btn)
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    app.dismiss_update();
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let update_btn = egui::Button::new(
                                            egui::RichText::new(format!(
                                                "{} {}",
                                                egui_phosphor::regular::DOWNLOAD_SIMPLE,
                                                t!("dialog.update.install")
                                            ))
                                            .size(15.0)
                                            .strong()
                                            .color(accent_text),
                                        )
                                        .fill(accent)
                                        .corner_radius(6.0)
                                        .min_size(egui::vec2(180.0, 40.0));

                                        if ui
                                            .add(update_btn)
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            app.start_update();
                                        }
                                    },
                                );
                            });
                        }
                    });

                ui.add_space(5.0);
            });
        });
}
