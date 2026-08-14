use crate::app::state::{TabletMapperApp, ToastLevel, UiSnapshot};
use crate::core::config::models::MappingConfig;
use crate::i18n::Locale;
use crate::t;
use crate::ui::theme::{panel_bg, panel_border, ui_input_box_u16, ui_input_box_u32};
use eframe::egui;

#[allow(clippy::too_many_lines)]
pub fn render_settings_panel(
    app: &mut TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    _snapshot: &UiSnapshot,
) {
    ui.add_space(15.0);
    render_general_settings(app, ui, config);

    ui.add_space(15.0);
    render_theme_settings(app, ui);

    ui.add_space(15.0);
    render_language_settings(app, ui);

    ui.add_space(15.0);
    render_websocket_settings(ui, config);
}

fn render_general_settings(
    app: &mut TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
) {
    render_card(
        ui,
        &t!("settings.general.title"),
        egui_phosphor::regular::GEAR_SIX,
        |ui| {
            let old_run_at_startup = config.run_at_startup;
            if ui
                .checkbox(
                    &mut config.run_at_startup,
                    t!("settings.general.run_at_startup"),
                )
                .on_hover_text(t!("settings.general.run_at_startup_tooltip"))
                .changed()
                && let Err(e) = crate::startup::set_run_at_startup(config.run_at_startup)
            {
                log::error!(target: "Config", "Failed to update startup setting: {e}");
                config.run_at_startup = old_run_at_startup;
            }

            ui.add_space(4.0);
            ui.checkbox(
                &mut config.system_tray_on_minimize,
                t!("settings.general.system_tray"),
            )
            .on_hover_text(t!("settings.general.system_tray_tooltip"));

            ui.add_space(4.0);
            if ui
                .checkbox(
                    &mut app.app_prefs.telemetry_enabled,
                    t!("settings.general.telemetry"),
                )
                .on_hover_text(t!("settings.general.telemetry_tooltip"))
                .changed()
            {
                crate::settings::app_preferences::save_app_preferences(&app.app_prefs);
            }
        },
    );
}

fn render_theme_settings(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    render_card(
        ui,
        &t!("settings.theme.title"),
        egui_phosphor::regular::PALETTE,
        |ui| {
            render_theme_selector(app, ui);
            ui.add_space(10.0);
            render_theme_external_actions(app, ui);
        },
    );

    crate::ui::components::theme_store::render_theme_store_viewport(app, ui);
}

fn render_theme_selector(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    let prefs = &mut app.app_prefs;
    let old_theme = prefs.theme.clone();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("settings.theme.label")).strong());
        ui.add_space(10.0);

        let theme_name = match &prefs.theme {
            crate::core::config::models::ThemePreference::Custom(name) => name.clone(),
            _ => format!("{:?}", prefs.theme),
        };

        egui::ComboBox::from_id_salt("theme_selector")
            .selected_text(theme_name)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut prefs.theme,
                    crate::core::config::models::ThemePreference::System,
                    "System",
                );
                ui.selectable_value(
                    &mut prefs.theme,
                    crate::core::config::models::ThemePreference::Light,
                    "Light",
                );
                ui.selectable_value(
                    &mut prefs.theme,
                    crate::core::config::models::ThemePreference::Dark,
                    "Dark",
                );
                ui.separator();
                ui.selectable_value(
                    &mut prefs.theme,
                    crate::core::config::models::ThemePreference::CatppuccinLatte,
                    "Catppuccin Latte",
                );
                ui.selectable_value(
                    &mut prefs.theme,
                    crate::core::config::models::ThemePreference::CatppuccinFrappe,
                    "Catppuccin Frappe",
                );
                ui.selectable_value(
                    &mut prefs.theme,
                    crate::core::config::models::ThemePreference::CatppuccinMacchiato,
                    "Catppuccin Macchiato",
                );
                ui.selectable_value(
                    &mut prefs.theme,
                    crate::core::config::models::ThemePreference::CatppuccinMocha,
                    "Catppuccin Mocha",
                );

                let custom_themes = crate::settings::themes::list_custom_themes();
                if !custom_themes.is_empty() {
                    ui.separator();
                    for name in custom_themes {
                        ui.selectable_value(
                            &mut prefs.theme,
                            crate::core::config::models::ThemePreference::Custom(name.clone()),
                            name,
                        );
                    }
                }
            });

        if let crate::core::config::models::ThemePreference::Custom(name) = &prefs.theme.clone() {
            let mut delete = false;
            ui.scope(|ui| {
                ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                if ui
                    .button(
                        egui::RichText::new(egui_phosphor::regular::TRASH)
                            .color(crate::ui::theme::semantic_colors(ui.ctx()).error),
                    )
                    .on_hover_text(t!("settings.general.delete_theme"))
                    .clicked()
                {
                    delete = true;
                }
            });
            if delete && crate::settings::themes::delete_custom_theme(name) == Ok(()) {
                prefs.theme = crate::core::config::models::ThemePreference::System;
            }
        }
    });

    if prefs.theme != old_theme {
        crate::ui::theme::apply_theme(ui.ctx(), &prefs.theme);
        crate::settings::app_preferences::save_app_preferences(prefs);
        crate::app::telemetry::capture_event_with_set(
            "theme_changed",
            Some(serde_json::json!({
                "previous_theme": format!("{old_theme:?}"),
                "new_theme": format!("{:?}", prefs.theme),
            })),
            Some(serde_json::json!({ "current_theme": format!("{:?}", prefs.theme) })),
        );
    }
}

fn render_theme_external_actions(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("settings.theme.download_title")).strong());
    });
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        if ui
            .button(format!(
                "{} {}",
                egui_phosphor::regular::GLOBE,
                t!("settings.theme.browse_online")
            ))
            .clicked()
        {
            app.theme_store.open = true;
            app.fetch_theme_store_list();
        }

        ui.add_space(10.0);

        if ui
            .button(format!(
                "{} {}",
                egui_phosphor::regular::DOWNLOAD_SIMPLE,
                t!("settings.general.import_theme")
            ))
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Theme File", &["json"])
                .pick_file()
        {
            match crate::settings::themes::import_theme_json(&path) {
                Ok(name) => {
                    app.app_prefs.theme =
                        crate::core::config::models::ThemePreference::Custom(name);
                    crate::ui::theme::apply_theme(ui.ctx(), &app.app_prefs.theme);
                    crate::settings::app_preferences::save_app_preferences(&app.app_prefs);
                }
                Err(e) => {
                    log::error!(target: "UI", "Failed to import theme: {e}");
                }
            }
        }
    });
}

fn render_language_settings(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    render_card(
        ui,
        &t!("settings.language.title"),
        egui_phosphor::regular::TRANSLATE,
        |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t!("settings.language.label")).strong());
                ui.add_space(10.0);

                let current_locale = app.app_prefs.language;
                let mut new_locale = current_locale;

                egui::ComboBox::from_id_salt("language_selector")
                    .selected_text(current_locale.display_name())
                    .show_ui(ui, |ui| {
                        for &locale in Locale::all() {
                            ui.selectable_value(&mut new_locale, locale, locale.display_name());
                        }
                    });

                if new_locale != current_locale {
                    app.app_prefs.language = new_locale;
                    crate::i18n::set_locale(new_locale);
                    crate::settings::app_preferences::save_app_preferences(&app.app_prefs);
                    crate::app::telemetry::capture_event_with_set(
                        "language_changed",
                        Some(serde_json::json!({
                            "previous_language": current_locale.display_name().to_string(),
                            "new_language": new_locale.display_name().to_string(),
                        })),
                        Some(serde_json::json!({
                            "language": new_locale.display_name().to_string(),
                        })),
                    );
                    app.push_toast(
                        t!(
                            "toast.language_changed",
                            language = new_locale.display_name()
                        ),
                        ToastLevel::Info,
                    );
                }
            });
        },
    );
}

fn render_websocket_settings(ui: &mut egui::Ui, config: &mut MappingConfig) {
    render_card(
        ui,
        &t!("settings.websocket.title"),
        egui_phosphor::regular::WIFI_HIGH,
        |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut config.websocket.enabled,
                    t!("settings.websocket.enable"),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let semantic = crate::ui::theme::semantic_colors(ui.ctx());
                    let (text, color) = if config.websocket.enabled {
                        (t!("settings.websocket.running"), semantic.success)
                    } else {
                        (t!("settings.websocket.stopped"), semantic.error)
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
                ui.label(
                    egui::RichText::new(t!("settings.websocket.payload"))
                        .weak()
                        .size(11.0),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut config.websocket.send_coordinates,
                        t!("settings.websocket.coords"),
                    );
                    ui.checkbox(
                        &mut config.websocket.send_pressure,
                        t!("settings.websocket.pressure"),
                    );
                    ui.checkbox(
                        &mut config.websocket.send_tilt,
                        t!("settings.websocket.tilt"),
                    );
                    ui.checkbox(
                        &mut config.websocket.send_status,
                        t!("settings.websocket.status"),
                    );
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
