use crate::app::state::{AppTab, TabletMapperApp};
use eframe::egui;

pub fn render_tabs(app: &mut TabletMapperApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("tabs")
        .frame(
            egui::Frame::new()
                .fill(ctx.style().visuals.panel_fill)
                .inner_margin(egui::Margin {
                    left: 5,
                    right: 5,
                    top: 5,
                    bottom: 5,
                })
                .stroke(egui::Stroke::NONE),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.active_tab, AppTab::Output, "Output");
                ui.selectable_value(&mut app.active_tab, AppTab::Filters, "Filters");
                ui.selectable_value(&mut app.active_tab, AppTab::PenSettings, "Pen Settings");
                ui.selectable_value(&mut app.active_tab, AppTab::Console, "Console");
                ui.selectable_value(&mut app.active_tab, AppTab::Settings, "Settings");
                ui.selectable_value(&mut app.active_tab, AppTab::Release, "Release");
            });
        });
}
