use crate::app::state::UiSnapshot;
use crate::t;
use eframe::egui;

pub fn render_debugger_panel(snapshot: &UiSnapshot, displayed_hz: f32, ui: &mut egui::Ui) {
    let tablet_data = &snapshot.tablet_data;
    let is_detected = snapshot.tablet_name != "No Tablet Detected";

    let (max_x, max_y, max_p) = (snapshot.hardware_size.0, snapshot.hardware_size.1, 8192.0);

    ui.add_space(10.0);

    let available_width = ui.available_width();
    let desired_height = (available_width * (9.0 / 16.0)).min(300.0);

    let (rect, _) = ui.allocate_at_least(
        egui::vec2(available_width, desired_height),
        egui::Sense::hover(),
    );

    ui.painter()
        .rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
    ui.painter().rect_stroke(
        rect,
        8.0,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Middle,
    );

    if is_detected && tablet_data.is_connected {
        let x_pct = f32::from(tablet_data.x) / max_x;
        let y_pct = f32::from(tablet_data.y) / max_y;

        let dot_pos = egui::pos2(
            x_pct.mul_add(rect.width(), rect.left()),
            y_pct.mul_add(rect.height(), rect.top()),
        );

        let is_down =
            tablet_data.status == crate::drivers::TabletStatus::Contact || tablet_data.pressure > 0;

        if is_down {
            ui.painter().circle_filled(
                dot_pos,
                10.0,
                ui.visuals().selection.bg_fill.gamma_multiply(0.2),
            );
            ui.painter()
                .circle_filled(dot_pos, 4.0, ui.visuals().selection.bg_fill);
        } else {
            ui.painter()
                .circle_filled(dot_pos, 3.0, ui.visuals().weak_text_color());
        }
    } else {
        let status_text = if is_detected {
            t!("debugger.pen_out_of_range")
        } else {
            t!("debugger.no_device")
        };

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            status_text,
            egui::FontId::proportional(16.0),
            ui.visuals().weak_text_color(),
        );
    }

    ui.add_space(20.0);

    let semantic = crate::ui::theme::semantic_colors(ui.ctx());

    ui.columns(2, |cols| {
        if let [col0, col1, ..] = cols {
            col0.vertical(|ui| {
                status_card(
                    ui,
                    &t!("debugger.report_status"),
                    tablet_data.status.as_str(),
                    semantic.success,
                );
                ui.add_space(10.0);
                status_card(
                    ui,
                    &t!("debugger.coordinates"),
                    &format!("X: {}, Y: {}", tablet_data.x, tablet_data.y),
                    ui.visuals().strong_text_color(),
                );
                ui.add_space(10.0);
                let tilt_str = format!("X: {}, Y: {}", tablet_data.tilt_x, tablet_data.tilt_y);
                status_card(
                    ui,
                    &t!("debugger.pen_tilt"),
                    &tilt_str,
                    ui.visuals().selection.bg_fill,
                );
            });
            col1.vertical(|ui| {
                status_card(
                    ui,
                    &t!("debugger.report_rate"),
                    &format!("{displayed_hz:.0} Hz"),
                    semantic.warning,
                );
                ui.add_space(10.0);
                status_card(
                    ui,
                    &t!("debugger.pressure"),
                    &format!("{} / {}", tablet_data.pressure, max_p as u16),
                    semantic.info,
                );
                ui.add_space(10.0);
                let b1 = (tablet_data.buttons & 0x01) != 0;
                let b2 = (tablet_data.buttons & 0x02) != 0;
                let btn_str = format!("B1: {b1} | B2: {b2}");
                status_card(
                    ui,
                    &t!("debugger.buttons"),
                    &btn_str,
                    if b1 || b2 {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().weak_text_color()
                    },
                );
            });
        }
    });

    ui.add_space(20.0);

    egui::Frame::group(ui.style())
        .fill(ui.visuals().widgets.noninteractive.bg_fill)
        .show(ui, |ui: &mut egui::Ui| {
            ui.set_width(ui.available_width());

            ui.label(
                egui::RichText::new(t!("debugger.raw_stream"))
                    .weak()
                    .size(11.0),
            );
            ui.label(
                egui::RichText::new(tablet_data.raw_hex())
                    .code()
                    .size(12.0)
                    .color(ui.visuals().text_color()),
            );

            ui.add_space(20.0);

            ui.label(
                egui::RichText::new(t!("debugger.raw_binary"))
                    .weak()
                    .size(11.0),
            );

            let mut binary_string = String::with_capacity(usize::from(tablet_data.raw_len) * 9);
            if let Some(raw_slice) = tablet_data.raw_data.get(..tablet_data.raw_len as usize) {
                for (i, &byte) in raw_slice.iter().enumerate() {
                    use std::fmt::Write;
                    if i > 0 {
                        let _ = write!(&mut binary_string, " ");
                    }
                    let _ = write!(&mut binary_string, "{byte:08b}");
                }
            }

            ui.label(
                egui::RichText::new(binary_string)
                    .code()
                    .size(12.0)
                    .color(ui.visuals().text_color()),
            );
        });
}

fn status_card(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    egui::Frame::new()
        .fill(ui.visuals().widgets.noninteractive.bg_fill)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui: &mut egui::Ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(label).weak().size(10.0));
            ui.label(egui::RichText::new(value).color(color).strong().size(17.0));
        });
}
