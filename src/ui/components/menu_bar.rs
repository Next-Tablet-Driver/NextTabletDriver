use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::t;
use eframe::egui;

pub fn render_menu_bar(app: &mut TabletMapperApp, ctx: &egui::Context, snapshot: &UiSnapshot) {
    egui::TopBottomPanel::top("menu_bar")
        .frame(
            egui::Frame::new()
                .fill(ctx.style().visuals.panel_fill)
                .inner_margin(5.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button(t!("menu.file"), |ui| {
                    if ui.button(t!("menu.file.load")).clicked() {
                        ui.close();
                        app.load_settings();
                    }

                    if ui.button(t!("menu.file.save")).clicked() {
                        ui.close();
                        app.save_settings(&snapshot.config);
                    }

                    if ui.button(t!("menu.file.save_as")).clicked() {
                        ui.close();
                        app.save_settings_as(snapshot.config.clone());
                    }

                    if ui.button(t!("menu.file.reset")).clicked() {
                        ui.close();
                        app.reset_to_default();
                    }

                    ui.separator();

                    if ui.button(t!("menu.file.export")).clicked() {
                        ui.close();
                        app.export_settings(&snapshot.config);
                    }
                    if ui.button(t!("menu.file.import")).clicked() {
                        ui.close();
                        app.import_settings();
                    }

                    if ui.button("Import OTD Settings").clicked() {
                        ui.close();
                        app.import_otd_settings();
                    }

                    ui.separator();

                    ui.menu_button(t!("menu.file.presets"), |ui| {
                        let profiles = crate::settings::list_profiles();
                        if profiles.is_empty() {
                            ui.label(
                                egui::RichText::new(t!("menu.file.no_presets"))
                                    .weak()
                                    .italics(),
                            );
                        } else {
                            for (name, path) in profiles {
                                if ui.button(name).clicked() {
                                    ui.close();
                                    app.load_profile_at_path(&path);
                                }
                            }
                        }
                    });
                });

                ui.menu_button(t!("menu.tablet"), |ui| {
                    if ui.button(t!("menu.tablet.debugger")).clicked() {
                        ui.close();
                        app.show_debugger = true;
                    }
                    if ui.button(t!("menu.tablet.input_lag")).clicked() {
                        ui.close();
                        app.show_latency_stats = true;
                    }
                });

                ui.menu_button(t!("menu.help"), |ui| {
                    if ui.button(t!("menu.help.github")).clicked() {
                        ui.close();
                        ui.ctx().open_url(egui::OpenUrl::new_tab(
                            "https://github.com/Next-Tablet-Driver/NextTabletDriver",
                        ));
                    }

                    // TODO: Report an issues -> Tablet Report / Others
                    // Multiple choice like `presets` function

                    ui.separator();

                    if ui.button(t!("menu.help.update")).clicked() {
                        ui.close();
                        app.check_for_updates();
                    }
                });
            });
        });
}
