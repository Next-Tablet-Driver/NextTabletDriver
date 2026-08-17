use crate::app::state::{AppTab, TabletMapperApp, UiSnapshot};
use crate::ui::panels::console::render_console_panel;
use crate::ui::panels::filters::render_filters_panel;
use crate::ui::panels::output::render_output_panel;
use crate::ui::panels::pen_settings::render_pen_settings_panel;
use crate::ui::panels::release::render_release_panel;
use crate::ui::panels::settings::render_settings_panel;
use eframe::egui;

impl TabletMapperApp {
    /// Renders the primary application interface.
    pub fn render_main_layout(
        &mut self,
        ctx: &egui::Context,
        config: &mut crate::core::config::models::MappingConfig,
        snapshot: &UiSnapshot,
    ) {
        if self.active_tab == AppTab::Release && self.previous_tab != AppTab::Release {
            self.request_release_notes_fetch(ctx);
        }
        self.previous_tab = self.active_tab;

        let (min_x, min_y, max_x, max_y) = self.calculate_display_bounds();

        crate::ui::components::menu_bar::render_menu_bar(self, ctx, snapshot);
        crate::ui::components::tabs::render_tabs(self, ctx);
        crate::ui::components::footer::render_footer(self, ctx, config, snapshot);

        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            AppTab::Output => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_output_panel(self, ui, config, snapshot, min_x, min_y, max_x, max_y);
                });
            }
            AppTab::Filters => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_filters_panel(self, ui, config, snapshot);
                });
            }
            AppTab::PenSettings => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_pen_settings_panel(self, ui, config, snapshot);
                });
            }
            AppTab::Console => render_console_panel(self, ui),
            AppTab::Settings => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_settings_panel(self, ui, config, snapshot);
                });
            }
            AppTab::Release => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_release_panel(self, ui);
                });
            }
        });
    }

    #[must_use]
    pub fn calculate_display_bounds(&self) -> (f32, f32, f32, f32) {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (0.0, 0.0, 1920.0, 1080.0);
        if !self.displays.is_empty() {
            let mut mx = i32::MAX;
            let mut my = i32::MAX;
            let mut ax = i32::MIN;
            let mut ay = i32::MIN;
            for d in &self.displays {
                mx = mx.min(d.x);
                my = my.min(d.y);
                ax = ax.max(d.x + d.width.cast_signed());
                ay = ay.max(d.y + d.height.cast_signed());
            }
            min_x = mx as f32;
            min_y = my as f32;
            max_x = ax as f32;
            max_y = ay as f32;
        }
        (min_x, min_y, max_x, max_y)
    }
}
