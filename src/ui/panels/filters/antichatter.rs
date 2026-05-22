use crate::core::config::models::MappingConfig;
use crate::ui::theme::{ui_card, ui_input_box, ui_setting_row};
use eframe::egui;

pub fn render_antichatter_settings(ui: &mut egui::Ui, config: &mut MappingConfig) {
    ui.add_space(5.0);

    ui_card(
        ui,
        "Smoothing & Antichatter",
        egui_phosphor::regular::WAVE_SINE,
        |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut config.antichatter.enabled, "Enable Antichatter");
            });
            ui.add_space(10.0);

            ui.add_enabled_ui(config.antichatter.enabled, |ui| {
                ui.vertical(|ui| {
                    ui_setting_row(ui, "Latency (ms)", &mut config.antichatter.latency, "");
                    ui_setting_row(
                        ui,
                        "Antichatter Strength",
                        &mut config.antichatter.antichatter_strength,
                        "",
                    );
                    ui_setting_row(
                        ui,
                        "Antichatter Multiplier",
                        &mut config.antichatter.antichatter_multiplier,
                        "",
                    );

                    ui.horizontal(|ui| {
                        ui_input_box(
                            ui,
                            "Offset X",
                            &mut config.antichatter.antichatter_offset_x,
                            "",
                        );
                        ui_input_box(
                            ui,
                            "Offset Y",
                            &mut config.antichatter.antichatter_offset_y,
                            "",
                        );
                    });

                    ui.separator();

                    ui_setting_row(
                        ui,
                        "Sampling Frequency",
                        &mut config.antichatter.frequency,
                        "Hz",
                    );
                });
            });
        },
    );

    ui.add_space(15.0);

    ui_card(
        ui,
        "Movement Prediction",
        egui_phosphor::regular::CHART_LINE_UP,
        |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut config.antichatter.prediction_enabled,
                    "Enable Prediction",
                );
            });
            ui.add_space(10.0);

            ui.add_enabled_ui(config.antichatter.prediction_enabled, |ui| {
                ui.vertical(|ui| {
                    ui_setting_row(
                        ui,
                        "Strength",
                        &mut config.antichatter.prediction_strength,
                        "",
                    );
                    ui_setting_row(
                        ui,
                        "Sharpness",
                        &mut config.antichatter.prediction_sharpness,
                        "",
                    );

                    ui.horizontal(|ui| {
                        ui_input_box(
                            ui,
                            "Offset X",
                            &mut config.antichatter.prediction_offset_x,
                            "",
                        );
                        ui_input_box(
                            ui,
                            "Offset Y",
                            &mut config.antichatter.prediction_offset_y,
                            "",
                        );
                    });
                });
            });
        },
    );
}
