use crate::app::state::TabletMapperApp;
use crate::t;
use eframe::egui;

pub fn render_console_panel(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    fn level_button(ui: &mut egui::Ui, selected: &mut bool, label: &str, color: egui::Color32) {
        let unselected_stroke = ui.visuals().widgets.noninteractive.bg_stroke.color;
        let stroke_color = if *selected {
            color.gamma_multiply(0.8)
        } else {
            unselected_stroke
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

    let semantic = crate::ui::theme::semantic_colors(ui.ctx());

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label(egui_phosphor::regular::MAGNIFYING_GLASS);
        ui.add(
            egui::TextEdit::singleline(&mut app.console_search)
                .hint_text(t!("console.search"))
                .desired_width(200.0),
        );
        if ui.button(egui_phosphor::regular::X).clicked() {
            app.console_search.clear();
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        level_button(ui, &mut app.console_show_info, "Info", semantic.info);
        level_button(ui, &mut app.console_show_warn, "Warn", semantic.warning);
        level_button(ui, &mut app.console_show_error, "Error", semantic.error);
        // Debug color: use a muted version of the text color (no dedicated semantic slot)
        let debug_color = ui.visuals().weak_text_color();
        level_button(ui, &mut app.console_show_debug, "Debug", debug_color);
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
            let semantic = crate::ui::theme::semantic_colors(ui.ctx());
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
                                ui.strong(t!("console.col.time"));
                            });
                            header.col(|ui| {
                                ui.strong(t!("console.col.level"));
                            });
                            header.col(|ui| {
                                ui.strong(t!("console.col.group"));
                            });
                            header.col(|ui| {
                                ui.strong(t!("console.col.message"));
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
                                            "Error" => (semantic.error, "ERROR"),
                                            "Warn" => (semantic.warning, "WARN"),
                                            "Info" => (semantic.info, "INFO"),
                                            "Debug" => (ui.visuals().weak_text_color(), "DEBUG"),
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
                                        // Wraps the log message in an invisible horizontal ScrollArea to allow
                                        // users to scroll very long messages using Shift+ScrollWheel while keeping
                                        // the preceding columns locked and visible.
                                        egui::ScrollArea::horizontal()
                                            .id_salt(index)
                                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                            .show(ui, |ui| {
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
    let semantic = crate::ui::theme::semantic_colors(ui.ctx());
    let mut toast_to_push = None;

    ui.horizontal(|ui| {
        if ui
            .button(
                egui::RichText::new(format!(
                    "{} {}",
                    egui_phosphor::regular::TRASH,
                    t!("console.clear")
                ))
                .color(semantic.error),
            )
            .on_hover_text(t!("console.clear_tooltip"))
            .clicked()
            && let Ok(mut entries) = crate::logger::LOG_BUFFER.write()
        {
            entries.clear();
            crate::logger::LOG_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        if ui
            .button(format!(
                "{} {}",
                egui_phosphor::regular::COPY,
                t!("console.copy_all")
            ))
            .on_hover_text(t!("console.copy_all_tooltip"))
            .clicked()
        {
            ui.output_mut(|o| {
                o.commands
                    .push(egui::OutputCommand::CopyText(full_log_text.to_string()));
            });
        }

        if ui
            .button(format!(
                "{} {}",
                egui_phosphor::regular::DOWNLOAD_SIMPLE,
                t!("console.export_logs")
            ))
            .on_hover_text(t!("console.export_logs_tooltip"))
            .clicked()
            && let Some(save_path) = rfd::FileDialog::new()
                .set_file_name("session_logs.txt")
                .add_filter("Text File", &["txt", "log"])
                .save_file()
        {
            let session_log_path = crate::settings::get_settings_dir().join("session.log");
            if let Err(e) = std::fs::copy(&session_log_path, &save_path) {
                toast_to_push = Some((
                    t!("toast.export_failed", error = e),
                    crate::app::state::ToastLevel::Error,
                ));
            } else {
                toast_to_push = Some((
                    t!("toast.logs_exported"),
                    crate::app::state::ToastLevel::Info,
                ));
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(t!(
                    "console.showing_logs",
                    shown = filtered_logs.len(),
                    total = all_logs_count
                ))
                .size(13.0)
                .color(ui.visuals().text_color().gamma_multiply(0.6)),
            );
        });
    });

    if let Some((msg, level)) = toast_to_push {
        app.push_toast(msg, level);
    }
}
