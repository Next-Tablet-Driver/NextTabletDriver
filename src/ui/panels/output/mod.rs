pub mod display;
pub mod relative;
pub mod tablet;

use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::core::config::models::MappingConfig;
use eframe::egui;

#[allow(clippy::too_many_arguments)]
pub fn render_output_panel(
    app: &TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    snapshot: &UiSnapshot,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) {
    ui.add_space(10.0);

    if config.mode == crate::core::config::models::DriverMode::Relative {
        relative::render_relative_mode_ui(ui, config);
        return;
    }

    display::render_display_section(app, ui, config, min_x, min_y, max_x, max_y);
    ui.add_space(20.0);
    tablet::render_tablet_section(ui, config, snapshot);
}
