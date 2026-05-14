use crate::app::state::TabletMapperApp;
use eframe::egui;

pub fn render_console_panel(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label(egui_phosphor::regular::MAGNIFYING_GLASS);
        ui.add(
            egui::TextEdit::singleline(&mut app.console_search)
                .hint_text("Search logs...")
                .desired_width(200.0),
        );
        if ui.button(egui_phosphor::regular::X).clicked() {
            app.console_search.clear();
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        fn level_button(ui: &mut egui::Ui, selected: &mut bool, label: &str, color: egui::Color32) {
            let stroke_color = if *selected {
                color.gamma_multiply(0.8)
            } else {
                egui::Color32::from_white_alpha(20)
            };
            let fill_color = if *selected {
                color.gamma_multiply(0.15)
            } else {
                egui::Color32::TRANSPARENT
            };
            let text_color = if *selected {
                color
            } else {
                ui.visuals().text_color().gamma_multiply(0.4)
            };

            let button = egui::Button::new(egui::RichText::new(label).color(text_color).strong())
                .fill(fill_color)
                .stroke(egui::Stroke::new(1.0, stroke_color))
                .corner_radius(4.0);

            if ui.add(button).clicked() {
                *selected = !*selected;
            }
        }

        level_button(
            ui,
            &mut app.console_show_info,
            "Info",
            egui::Color32::from_rgb(137, 180, 250),
        );
        level_button(
            ui,
            &mut app.console_show_warn,
            "Warn",
            egui::Color32::from_rgb(249, 226, 175),
        );
        level_button(
            ui,
            &mut app.console_show_error,
            "Error",
            egui::Color32::from_rgb(243, 139, 168),
        );
        level_button(
            ui,
            &mut app.console_show_debug,
            "Debug",
            egui::Color32::from_rgb(166, 172, 205),
        );
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(2.0);

    let (all_logs_count, filtered_logs, full_log_text) = app.get_filtered_logs();

    let footer_height = 45.0;
    let table_height = ui.available_height() - footer_height;

    // 1. Table Area
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), table_height),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            egui::Frame::new()
                .fill(ui.visuals().window_fill)
                .inner_margin(0.0)
                .show(ui, |ui| {
                    use egui_extras::{Column, TableBuilder};

                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .vscroll(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::initial(85.0).at_least(85.0)) // Time
                        .column(Column::initial(65.0).at_least(60.0)) // Level
                        .column(Column::initial(130.0).at_least(110.0)) // Group
                        .column(Column::remainder().at_least(250.0)) // Message
                        .header(25.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("Time");
                            });
                            header.col(|ui| {
                                ui.strong("Level");
                            });
                            header.col(|ui| {
                                ui.strong("Group");
                            });
                            header.col(|ui| {
                                ui.strong("Message");
                            });
                        })
                        .body(|body| {
                            body.rows(24.0, filtered_logs.len(), |mut row| {
                                let index = row.index();
                                if let Some(log) = filtered_logs.get(index) {
                                    row.col(|ui| {
                                        ui.label(
                                            egui::RichText::new(&log.time).monospace().size(13.0),
                                        );
                                    });

                                    row.col(|ui| {
                                        let (color, text) = match log.level.as_str() {
                                            "Error" => {
                                                (egui::Color32::from_rgb(243, 139, 168), "ERROR")
                                            }
                                            "Warn" => {
                                                (egui::Color32::from_rgb(249, 226, 175), "WARN")
                                            }
                                            "Info" => {
                                                (egui::Color32::from_rgb(137, 180, 250), "INFO")
                                            }
                                            "Debug" => {
                                                (egui::Color32::from_rgb(166, 172, 205), "DEBUG")
                                            }
                                            _ => (ui.visuals().text_color(), log.level.as_str()),
                                        };
                                        ui.label(
                                            egui::RichText::new(text)
                                                .color(color)
                                                .strong()
                                                .size(12.0),
                                        );
                                    });

                                    row.col(|ui| {
                                        ui.label(
                                            egui::RichText::new(&log.group)
                                                .color(ui.visuals().strong_text_color())
                                                .size(13.0),
                                        );
                                    });

                                    row.col(|ui| {
                                        let label = ui.label(
                                            egui::RichText::new(&log.message)
                                                .monospace()
                                                .size(13.0)
                                                .color(ui.visuals().text_color()),
                                        );
                                        if log.message.len() > 50 {
                                            label.on_hover_text(&log.message);
                                        }
                                    });
                                }
                            });
                        });
                });
        },
    );

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(5.0);

    // 2. Footer Area
    ui.horizontal(|ui| {
        if ui
            .button(
                egui::RichText::new(format!("{} Clear Console", egui_phosphor::regular::TRASH))
                    .color(egui::Color32::from_rgb(243, 139, 168)),
            )
            .on_hover_text("Remove all logs from memory")
            .clicked()
            && let Ok(mut entries) = crate::logger::LOG_BUFFER.write()
        {
            entries.clear();
        }

        if ui
            .button(format!(
                "{} Copy Unfiltered Logs",
                egui_phosphor::regular::COPY
            ))
            .clicked()
        {
            ui.output_mut(|o| {
                o.commands
                    .push(egui::OutputCommand::CopyText(full_log_text.to_string()));
            });
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Showing {} / {} logs",
                    filtered_logs.len(),
                    all_logs_count
                ))
                .size(13.0)
                .color(ui.visuals().text_color().gamma_multiply(0.6)),
            );
        });
    });
}
