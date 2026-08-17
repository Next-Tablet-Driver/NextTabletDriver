use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::t;
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
                    .selected_text(t!(
                        "footer.mode.current",
                        mode = format!("{current_mode:?}")
                    ))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut current_mode,
                            crate::core::config::models::DriverMode::Absolute,
                            t!("footer.mode.absolute"),
                        );
                        ui.selectable_value(
                            &mut current_mode,
                            crate::core::config::models::DriverMode::Relative,
                            t!("footer.mode.relative"),
                        );
                    });

                if current_mode != config.mode {
                    crate::app::telemetry::capture_event_with_set(
                        "driver_mode_changed",
                        Some(serde_json::json!({
                            "previous_mode": format!("{:?}", config.mode),
                            "new_mode": format!("{current_mode:?}"),
                        })),
                        Some(serde_json::json!({ "driver_mode": format!("{current_mode:?}") })),
                    );
                    config.mode = current_mode;
                    app.shared.config_version.fetch_add(1, Ordering::SeqCst);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("V{}", crate::VERSION))
                            .color(ui.visuals().weak_text_color())
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
                        ui.label(t!("footer.profile"));
                    });

                    egui::ComboBox::from_id_salt("device_combo")
                        .width(200.0)
                        .selected_text(tablet_name)
                        .show_ui(ui, |_| {});
                });
            });
        });
}
