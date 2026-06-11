use crate::app::state::{AppTab, TabletMapperApp};
use crate::t;
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
                ui.selectable_value(&mut app.active_tab, AppTab::Output, t!("tabs.output"));
                ui.selectable_value(&mut app.active_tab, AppTab::Filters, t!("tabs.filters"));
                ui.selectable_value(
                    &mut app.active_tab,
                    AppTab::PenSettings,
                    t!("tabs.pen_settings"),
                );
                ui.selectable_value(&mut app.active_tab, AppTab::Console, t!("tabs.console"));
                ui.selectable_value(&mut app.active_tab, AppTab::Settings, t!("tabs.settings"));
                ui.selectable_value(&mut app.active_tab, AppTab::Release, t!("tabs.release"));
            });
        });
}
