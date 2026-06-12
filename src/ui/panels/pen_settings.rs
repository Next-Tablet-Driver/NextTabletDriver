use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::core::config::models::MappingConfig;
use crate::t;
use crate::ui::theme::{panel_border, ui_card, ui_input_box_u16_range};
use eframe::egui;

pub fn render_pen_settings_panel(
    _app: &TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    _snapshot: &UiSnapshot,
) {
    ui.add_space(15.0);

    ui_card(ui, &t!("pen.title"), egui_phosphor::regular::PEN, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(t!("pen.thresholds")).weak().size(11.0));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui_input_box_u16_range(ui, "Tip", &mut config.tip_threshold, "", 1..=100);
                    ui.add_space(10.0);
                    ui_input_box_u16_range(ui, "Eraser", &mut config.eraser_threshold, "", 1..=100);
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(t!("pen.hardware_toggles"))
                            .weak()
                            .size(11.0),
                    );
                    ui.add_space(8.0);
                    ui.checkbox(&mut config.disable_pressure, t!("pen.disable_pressure"));
                    ui.checkbox(&mut config.disable_tilt, t!("pen.disable_tilt"));
                });
            });
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            render_binding_row(ui, &t!("pen.tip_binding"), &config.tip_binding, "tip_edit");
            ui.add_space(20.0);
            render_binding_row(
                ui,
                &t!("pen.eraser_binding"),
                &config.eraser_binding,
                "eraser_edit",
            );
        });
    });

    ui.add_space(15.0);

    ui_card(
        ui,
        &t!("pen.button_actions"),
        egui_phosphor::regular::MOUSE,
        |ui| {
            ui.vertical(|ui| {
                for i in 0..2 {
                    if let Some(binding) = config.pen_button_bindings.get(i) {
                        render_binding_row(
                            ui,
                            &t!("pen.pen_button", index = i + 1),
                            binding,
                            &format!("btn_edit_{i}"),
                        );
                        if i == 0 {
                            ui.add_space(8.0);
                        }
                    }
                }
            });
        },
    );
}

fn render_binding_row(ui: &mut egui::Ui, label: &str, binding: &str, _id: &str) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).strong().size(12.0));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let visuals = ui.visuals();
            egui::Frame::new()
                .fill(visuals.widgets.noninteractive.bg_fill)
                .stroke(egui::Stroke::new(
                    1.0,
                    panel_border(visuals).gamma_multiply(0.5),
                ))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.set_min_width(200.0);
                    ui.label(egui::RichText::new(binding).monospace().size(11.0));
                });

            if ui
                .button(egui_phosphor::regular::PENCIL_SIMPLE)
                .on_hover_text(t!("pen.edit_binding"))
                .clicked()
            {}
        });
    });
}
