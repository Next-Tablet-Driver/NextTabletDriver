use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::core::config::models::MappingConfig;
use crate::t;
use crate::ui::theme::{panel_border, ui_card, ui_input_box_u16};
use eframe::egui;

pub fn render_stats_settings(
    app: &TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    snapshot: &UiSnapshot,
) {
    let stats = &snapshot.stats;

    ui.add_space(5.0);

    ui_card(
        ui,
        &t!("filters.stats.live"),
        egui_phosphor::regular::GAUGE,
        |ui| {
            ui.horizontal(|ui| {
                render_stat_badge(
                    ui,
                    &t!("filters.stats.handspeed"),
                    &format!("{:.2}", stats.handspeed),
                    match config.speed_stats.unit {
                        crate::core::config::models::SpeedUnit::MillimetersPerSecond => "mm/s",
                        crate::core::config::models::SpeedUnit::MetersPerSecond => "m/s",
                        crate::core::config::models::SpeedUnit::KilometersPerHour => "km/h",
                        crate::core::config::models::SpeedUnit::MilesPerHour => "mph",
                    },
                );

                ui.add_space(15.0);

                let (dist_val, dist_unit) = stats.format_distance();
                render_stat_badge(
                    ui,
                    &t!("filters.stats.total_distance"),
                    &dist_val,
                    dist_unit,
                );

                ui.add_space(15.0);

                if ui
                    .button(egui_phosphor::regular::ARROWS_COUNTER_CLOCKWISE)
                    .on_hover_text(t!("filters.stats.reset_distance"))
                    .clicked()
                    && let Ok(mut stats) = app.shared.stats.write()
                {
                    stats.reset_distance();
                }
            });
        },
    );

    ui.add_space(15.0);

    ui_card(
        ui,
        &t!("filters.stats.server_config"),
        egui_phosphor::regular::GLOBE,
        |ui| {
            ui.horizontal(|ui| {
                if ui
                    .checkbox(
                        &mut config.speed_stats.enabled,
                        t!("filters.stats.enable_server"),
                    )
                    .changed()
                    && config.speed_stats.enabled
                {
                    let app_prefs = crate::settings::app_preferences::load_app_preferences();
                    crate::app::telemetry::capture_event(
                        "filter_enabled",
                        Some(serde_json::json!({"filter_name": "HandSpeed"})),
                        &app_prefs,
                    );
                }
            });

            ui.add_space(12.0);

            ui.add_enabled_ui(config.speed_stats.enabled, |ui| {
                ui.horizontal(|ui| {
                    crate::ui::theme::ui_input_box_string(
                        ui,
                        "IP Address",
                        &mut config.speed_stats.ip,
                        180.0,
                    );

                    ui.add_space(8.0);

                    ui_input_box_u16(ui, "Port", &mut config.speed_stats.port, "");

                    ui.add_space(8.0);

                    crate::ui::theme::ui_labeled_box(
                        ui,
                        &t!("filters.stats.speed_unit"),
                        170.0,
                        |ui| {
                            egui::ComboBox::from_id_salt("speed_unit_combo")
                            .selected_text(match config.speed_stats.unit {
                                crate::core::config::models::SpeedUnit::MillimetersPerSecond => {
                                    "mm/s"
                                }
                                crate::core::config::models::SpeedUnit::MetersPerSecond => "m/s",
                                crate::core::config::models::SpeedUnit::KilometersPerHour => "km/h",
                                crate::core::config::models::SpeedUnit::MilesPerHour => "mph",
                            })
                            .show_ui(ui, |ui: &mut egui::Ui| {
                                ui.selectable_value(
                                    &mut config.speed_stats.unit,
                                    crate::core::config::models::SpeedUnit::MillimetersPerSecond,
                                    "mm/s",
                                );
                                ui.selectable_value(
                                    &mut config.speed_stats.unit,
                                    crate::core::config::models::SpeedUnit::MetersPerSecond,
                                    "m/s",
                                );
                                ui.selectable_value(
                                    &mut config.speed_stats.unit,
                                    crate::core::config::models::SpeedUnit::KilometersPerHour,
                                    "km/h",
                                );
                                ui.selectable_value(
                                    &mut config.speed_stats.unit,
                                    crate::core::config::models::SpeedUnit::MilesPerHour,
                                    "mph",
                                );
                            });
                        },
                    );
                });
            });
        },
    );

    ui.add_space(15.0);

    ui_card(
        ui,
        &t!("filters.stats.docs"),
        egui_phosphor::regular::INFO,
        |ui| {
            ui.label(
                egui::RichText::new(t!(
                    "filters.stats.listening_at",
                    ip = &config.speed_stats.ip,
                    port = config.speed_stats.port
                ))
                .strong(),
            );
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new(t!("filters.stats.payload_format"))
                    .weak()
                    .size(11.0),
            );
            ui.add_space(4.0);

            egui::Frame::new()
            .fill(ui.visuals().widgets.noninteractive.bg_fill)
            .stroke(egui::Stroke::new(1.0, panel_border(ui.visuals()).gamma_multiply(0.5)))
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("{ \"handspeed\": float, \"timestamp\": u128, \"total_distance\": float }").monospace().size(10.0));
            });
        },
    );
}

fn render_stat_badge(ui: &mut egui::Ui, label: &str, value: &str, unit: &str) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).weak().size(10.0));
        ui.add_space(2.0);

        let accent = ui.visuals().selection.bg_fill;

        egui::Frame::new()
            .fill(accent.gamma_multiply(0.1))
            .stroke(egui::Stroke::new(1.0, accent.gamma_multiply(0.3)))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(10, 5))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(value).color(accent).strong().size(15.0));
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(unit)
                            .color(accent.gamma_multiply(0.6))
                            .size(10.0),
                    );
                });
            });
    });
}
