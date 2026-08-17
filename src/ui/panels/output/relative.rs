use crate::core::config::models::MappingConfig;
use crate::t;
use crate::ui::theme::{ui_input_box, ui_input_box_u32, ui_section_header};
use eframe::egui;

pub fn render_relative_mode_ui(ui: &mut egui::Ui, config: &mut MappingConfig) {
    ui_section_header(ui, &t!("output.relative"));

    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui_input_box(
            ui,
            "X Sensitivity",
            &mut config.relative_config.x_sensitivity,
            "px/mm",
        );
        ui_input_box(
            ui,
            "Y Sensitivity",
            &mut config.relative_config.y_sensitivity,
            "px/mm",
        );
        ui_input_box(ui, "Rotation", &mut config.relative_config.rotation, "°");
        ui_input_box_u32(
            ui,
            "Reset Time",
            &mut config.relative_config.reset_time_ms,
            "ms",
        );
    });
    ui.add_space(20.0);
}
