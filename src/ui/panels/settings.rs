use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::core::config::models::MappingConfig;
use crate::ui::theme::{panel_bg, panel_border, ui_input_box_u16, ui_input_box_u32};
use eframe::egui;

#[allow(clippy::too_many_lines)]
pub fn render_settings_panel(
    _app: &TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    _snapshot: &UiSnapshot,
) {
    ui.add_space(15.0);

    render_card(
        ui,
        "General Settings",
        egui_phosphor::regular::GEAR_SIX,
        |ui| {
            let old_run_at_startup = config.run_at_startup;
            if ui
                .checkbox(&mut config.run_at_startup, "Run at startup")
                .on_hover_text("Automatically launch the application when your session starts.")
                .changed()
                && let Err(e) = crate::startup::set_run_at_startup(config.run_at_startup)
            {
                log::error!(target: "Config", "Failed to update startup setting: {e}");
                config.run_at_startup = old_run_at_startup;
            }

            ui.add_space(4.0);
            ui.checkbox(
                &mut config.system_tray_on_minimize,
                "System Tray when Minimize",
            )
            .on_hover_text("Hide the application to the system tray when minimized.");

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Application Theme").strong());
                ui.add_space(10.0);

                let theme_name = match &config.theme {
                    crate::core::config::models::ThemePreference::Custom(name) => name.clone(),
                    _ => format!("{:?}", config.theme),
                };

                egui::ComboBox::from_id_salt("theme_selector")
                    .selected_text(theme_name)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut config.theme,
                            crate::core::config::models::ThemePreference::System,
                            "System",
                        );
                        ui.selectable_value(
                            &mut config.theme,
                            crate::core::config::models::ThemePreference::Light,
                            "Light",
                        );
                        ui.selectable_value(
                            &mut config.theme,
                            crate::core::config::models::ThemePreference::Dark,
                            "Dark",
                        );
                        ui.separator();
                        ui.selectable_value(
                            &mut config.theme,
                            crate::core::config::models::ThemePreference::CatppuccinLatte,
                            "Catppuccin Latte",
                        );
                        ui.selectable_value(
                            &mut config.theme,
                            crate::core::config::models::ThemePreference::CatppuccinFrappe,
                            "Catppuccin Frappe",
                        );
                        ui.selectable_value(
                            &mut config.theme,
                            crate::core::config::models::ThemePreference::CatppuccinMacchiato,
                            "Catppuccin Macchiato",
                        );
                        ui.selectable_value(
                            &mut config.theme,
                            crate::core::config::models::ThemePreference::CatppuccinMocha,
                            "Catppuccin Mocha",
                        );
                        
                        let custom_themes = crate::settings::themes::list_custom_themes();
                        if !custom_themes.is_empty() {
                            ui.separator();
                            for name in custom_themes {
                                ui.selectable_value(
                                    &mut config.theme,
                                    crate::core::config::models::ThemePreference::Custom(name.clone()),
                                    name,
                                );
                            }
                        }
                    });

                if let crate::core::config::models::ThemePreference::Custom(name) = &config.theme.clone() {
                    let mut delete = false;
                    ui.scope(|ui| {
                        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        if ui
                            .button(
                                egui::RichText::new(egui_phosphor::regular::TRASH)
                                    .color(crate::ui::theme::semantic_colors(ui.ctx()).error),
                            )
                            .on_hover_text("Delete custom theme")
                            .clicked()
                        {
                            delete = true;
                        }
                    });
                    if delete {
                        if let Ok(_) = crate::settings::themes::delete_custom_theme(name) {
                            config.theme = crate::core::config::models::ThemePreference::System;
                        }
                    }
                }
                    
                ui.add_space(10.0);
                if ui.button(format!("{} Import Theme (.json)", egui_phosphor::regular::DOWNLOAD_SIMPLE)).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Theme File", &["json"])
                        .pick_file() 
                    {
                        match crate::settings::themes::import_theme_json(&path) {
                            Ok(name) => {
                                config.theme = crate::core::config::models::ThemePreference::Custom(name);
                            }
                            Err(e) => {
                                log::error!(target: "UI", "Failed to import theme: {e}");
                            }
                        }
                    }
                }
            });
        },
    );

    ui.add_space(15.0);

    render_card(
        ui,
        "WebSocket Server",
        egui_phosphor::regular::WIFI_HIGH,
        |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut config.websocket.enabled, "Enable WebSocket Server");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let semantic = crate::ui::theme::semantic_colors(ui.ctx());
                    let (text, color) = if config.websocket.enabled {
                        ("RUNNING", semantic.success)
                    } else {
                        ("STOPPED", semantic.error)
                    };

                    egui::Frame::new()
                        .fill(color.gamma_multiply(0.1))
                        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.5)))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(text).color(color).size(10.0).strong());
                        });
                });
            });

            ui.add_space(12.0);

            ui.add_enabled_ui(config.websocket.enabled, |ui| {
                ui.horizontal(|ui| {
                    ui_input_box_u16(ui, "Port", &mut config.websocket.port, "");
                    ui.add_space(10.0);
                    ui_input_box_u32(ui, "Rate", &mut config.websocket.polling_rate_hz, "Hz");
                });

                ui.add_space(15.0);
                ui.label(egui::RichText::new("Payload Data").weak().size(11.0));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut config.websocket.send_coordinates, "Coords");
                    ui.checkbox(&mut config.websocket.send_pressure, "Pressure");
                    ui.checkbox(&mut config.websocket.send_tilt, "Tilt");
                    ui.checkbox(&mut config.websocket.send_status, "Status");
                });
            });
        },
    );
}

fn render_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    icon: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    let visuals = ui.visuals();
    let card_bg = panel_bg(visuals).gamma_multiply(0.6);
    let border_color = panel_border(visuals).gamma_multiply(0.4);

    egui::Frame::new()
        .fill(card_bg)
        .corner_radius(4.0)
        .stroke(egui::Stroke::new(1.0, border_color))
        .inner_margin(egui::Margin::symmetric(20, 15))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{icon} {title}"))
                            .size(15.0)
                            .strong(),
                    );
                });

                ui.add_space(12.0);
                add_contents(ui);
            });
        });
}
