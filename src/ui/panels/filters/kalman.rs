use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::core::config::models::MappingConfig;
use crate::t;
use crate::ui::theme::{ui_card, ui_setting_row_range};
use eframe::egui;

pub fn render_kalman_settings(
    _app: &TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    _snapshot: &UiSnapshot,
) {
    ui.add_space(5.0);

    ui_card(
        ui,
        &t!("filters.kalman.title"),
        egui_phosphor::regular::WAVEFORM,
        |ui| {
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut config.kalman.enabled, t!("filters.kalman.enable"))
                    .changed()
                {
                    crate::app::telemetry::capture_event(
                        "filter_toggled",
                        Some(serde_json::json!({
                            "filter_name": "Kalman",
                            "enabled": config.kalman.enabled,
                        })),
                    );
                }
            });
            ui.add_space(10.0);

            ui.add_enabled_ui(config.kalman.enabled, |ui| {
                ui.vertical(|ui| {
                    ui_setting_row_range(
                        ui,
                        &t!("filters.kalman.process_noise"),
                        &mut config.kalman.process_noise,
                        "",
                        0.0001..=10.0,
                    );
                    ui_setting_row_range(
                        ui,
                        &t!("filters.kalman.measurement_noise"),
                        &mut config.kalman.measurement_noise,
                        "",
                        0.0001..=10.0,
                    );
                });
            });
        },
    );
}
