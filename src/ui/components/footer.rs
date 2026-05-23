use crate::app::state::{TabletMapperApp, UiSnapshot};
use eframe::egui;
use std::sync::atomic::Ordering;

pub fn render_footer(
    app: &mut TabletMapperApp,
    ctx: &egui::Context,
    config: &mut crate::core::config::models::MappingConfig,
    snapshot: &UiSnapshot,
) {
    let tablet_name = &snapshot.tablet_name;
    let profile_display = app.profile.display_name(config);

    egui::TopBottomPanel::bottom("footer")
        .frame(
            egui::Frame::new()
                .fill(ctx.style().visuals.panel_fill)
                .inner_margin(egui::Margin::symmetric(10, 5))
                .stroke(ctx.style().visuals.widgets.noninteractive.bg_stroke),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut current_mode = config.mode;
                egui::ComboBox::from_id_salt("mode_combo")
                    .selected_text(format!("{current_mode:?} Mode"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut current_mode,
                            crate::core::config::models::DriverMode::Absolute,
                            "Absolute Mode",
                        );
                        ui.selectable_value(
                            &mut current_mode,
                            crate::core::config::models::DriverMode::Relative,
                            "Relative Mode",
                        );
                    });

                if current_mode != config.mode {
                    config.mode = current_mode;
                    app.shared.config_version.fetch_add(1, Ordering::SeqCst);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("V{}", crate::VERSION))
                            .color(egui::Color32::GRAY)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        let label_text = if app.profile.is_dirty(config) {
                            egui::RichText::new(&profile_display).strong().italics()
                        } else {
                            egui::RichText::new(&profile_display).strong()
                        };
                        ui.label(label_text);
                        ui.label("Profile:");
                    });

                    egui::ComboBox::from_id_salt("device_combo")
                        .width(200.0)
                        .selected_text(tablet_name)
                        .show_ui(ui, |_| {});
                });
            });
        });
}
