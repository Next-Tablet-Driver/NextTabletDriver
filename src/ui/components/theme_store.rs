use crate::app::state::TabletMapperApp;
use crate::t;
use eframe::egui;

pub fn render_theme_store_viewport(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    if !app.theme_store.open {
        return;
    }

    let viewport_id = egui::ViewportId::from_hash_of("theme_store_viewport");
    let builder = egui::ViewportBuilder::default()
        .with_title(t!("settings.theme.store_title"))
        .with_inner_size([450.0, 500.0])
        .with_min_inner_size([300.0, 300.0]);

    ui.ctx()
        .show_viewport_immediate(viewport_id, builder, |ctx, _class| {
            // Inherit style and semantic colors from the main context
            // to avoid re-parsing the theme.json file and spamming logs every frame.
            ctx.set_style(ui.ctx().style());
            let semantic = crate::ui::theme::semantic_colors(ui.ctx());
            ctx.memory_mut(|mem| {
                mem.data
                    .insert_temp(egui::Id::new("semantic_colors"), semantic);
            });
            if ctx.input(|i| i.viewport().close_requested()) {
                app.theme_store.open = false;
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                render_theme_store_content(app, ui);
            });
        });
}

fn render_theme_store_content(app: &mut TabletMapperApp, ui: &mut egui::Ui) {
    let semantic = crate::ui::theme::semantic_colors(ui.ctx());

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(t!("settings.theme.store_title"))
                .strong()
                .size(16.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            if ui
                .button(egui_phosphor::regular::ARROWS_CLOCKWISE)
                .on_hover_text("Refresh")
                .clicked()
            {
                if let Ok(mut lock) = app.theme_store.list.lock() {
                    *lock = None;
                }
                app.fetch_theme_store_list();
            }
        });
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let search_icon = egui::RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS);
        ui.label(search_icon);
        let _ = ui.add(
            egui::TextEdit::singleline(&mut app.theme_store.search)
                .hint_text(t!(
                    "settings.theme.search_hint",
                    default = "Search themes..."
                ))
                .desired_width(150.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            egui::ComboBox::from_id_salt("theme_filter")
                .selected_text(match app.theme_store.filter_mode {
                    None => t!("settings.theme.filter_all", default = "All"),
                    Some(true) => t!("settings.theme.filter_dark", default = "Dark"),
                    Some(false) => t!("settings.theme.filter_light", default = "Light"),
                })
                .width(80.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.theme_store.filter_mode,
                        None,
                        t!("settings.theme.filter_all", default = "All"),
                    );
                    ui.selectable_value(
                        &mut app.theme_store.filter_mode,
                        Some(true),
                        t!("settings.theme.filter_dark", default = "Dark"),
                    );
                    ui.selectable_value(
                        &mut app.theme_store.filter_mode,
                        Some(false),
                        t!("settings.theme.filter_light", default = "Light"),
                    );
                });
            ui.label(egui::RichText::new(egui_phosphor::regular::FUNNEL));
        });
    });
    ui.add_space(8.0);
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let themes_state = if let Ok(lock) = app.theme_store.list.lock() {
                (*lock).clone()
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Failed to acquire lock on theme store list")
                            .color(semantic.error),
                    );
                });
                return;
            };

            match themes_state {
                None => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Loading...").weak());
                    });
                }
                Some(Err(e)) => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(
                            egui::RichText::new(egui_phosphor::regular::WARNING_CIRCLE)
                                .size(32.0)
                                .color(semantic.error),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Failed to load themes")
                                .strong()
                                .color(semantic.error),
                        );
                        ui.label(e);
                    });
                }
                Some(Ok(themes)) => {
                    let local_themes = crate::settings::themes::list_custom_themes();
                    let search_lower = app.theme_store.search.to_lowercase();
                    let filtered_themes: Vec<_> = themes.iter().filter(|item| {
                        if !search_lower.is_empty() {
                            let matches_name = item.metadata.name.to_lowercase().contains(&search_lower);
                            let matches_author = item.metadata.author.to_lowercase().contains(&search_lower);
                            if !matches_name && !matches_author {
                                return false;
                            }
                        }
                        if let Some(mode) = app.theme_store.filter_mode && item.dark_mode != mode {
                            return false;
                        }
                        true
                    }).collect();
                    ui.add_space(5.0);
                    for item in filtered_themes {
                        let theme = &item.metadata;
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::PALETTE)
                                    .color(if local_themes.contains(&theme.name) { semantic.success } else { semantic.info })
                            );
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&theme.name).strong());
                                ui.label(egui::RichText::new(format!("{} • v{}", theme.author, theme.version)).weak().size(11.0));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add_space(12.0);
                                if local_themes.contains(&theme.name) {
                                    if ui.button(t!("settings.theme.apply", default = "Apply")).clicked() {
                                        app.app_prefs.theme = crate::core::config::models::ThemePreference::Custom(theme.name.clone());
                                        crate::ui::theme::apply_theme(ui.ctx(), &app.app_prefs.theme);
                                        crate::settings::app_preferences::save_app_preferences(&app.app_prefs);
                                    }
                                    ui.add_enabled_ui(false, |ui| {
                                        let _ = ui.button(t!("settings.theme.installed"));
                                    });
                                } else if Some(&theme.name) == app.theme_store.downloading_name.as_ref() {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.add_enabled_ui(false, |ui| {
                                            let _ = ui.button(t!("settings.theme.downloading", default = "Downloading..."));
                                        });
                                    });
                                } else {
                                    let is_any_downloading = app.theme_store.downloading_name.is_some();
                                    ui.add_enabled_ui(!is_any_downloading, |ui| {
                                        let btn = ui.button(format!(
                                            "{} {}",
                                            egui_phosphor::regular::DOWNLOAD_SIMPLE,
                                            t!("settings.theme.download_btn")
                                        ));
                                        if btn.clicked() {
                                            app.download_theme(&theme.name, ui.ctx());
                                        }
                                    });
                                }
                            });
                        });
                        ui.add_space(5.0);
                        ui.separator();
                        ui.add_space(5.0);
                    }
                }
            }
        });
}
