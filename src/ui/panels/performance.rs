use crate::app::state::UiSnapshot;
use crate::engine::state::SharedState;
use eframe::egui;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn render_performance_panel(
    snapshot: &UiSnapshot,
    displayed_hz: f32,
    ui_latency: f32,
    min_ui_latency: f32,
    max_ui_latency: f32,
    avg_ui_latency: f32,
    ui: &mut egui::Ui,
    shared: Arc<SharedState>,
) -> bool {
    let tablet_data = &snapshot.tablet_data;
    let stats = &snapshot.stats;
    let (max_w, max_h) = snapshot.hardware_size;
    let mut reset_requested = false;

    ui.add_space(10.0);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Latency Pipeline Analysis")
                    .strong()
                    .size(14.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Reset Stats").clicked() {
                    if let Ok(mut s) = shared.stats.write() {
                        s.reset_latency();
                    }
                    reset_requested = true;
                }
            });
        });
        ui.add_space(5.0);

        egui::Grid::new("latency_grid_refined")
            .num_columns(5)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Component");
                ui.label("Current");
                ui.label("AVG (EMA)");
                ui.label("Min");
                ui.label("Max");
                ui.end_row();

                ui.label("HID Read:");
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.hid_read_ms))
                        .color(egui::Color32::LIGHT_BLUE),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.avg_hid_read_ms))
                        .color(egui::Color32::LIGHT_BLUE)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}ms",
                        if stats.min_hid_read_ms == f32::MAX {
                            0.0
                        } else {
                            stats.min_hid_read_ms
                        }
                    ))
                    .weak(),
                );
                ui.label(egui::RichText::new(format!("{:.3}ms", stats.max_hid_read_ms)).weak());
                ui.end_row();

                ui.label("Parser:");
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.parser_ms))
                        .color(egui::Color32::LIGHT_GREEN),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.avg_parser_ms))
                        .color(egui::Color32::LIGHT_GREEN)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}ms",
                        if stats.min_parser_ms == f32::MAX {
                            0.0
                        } else {
                            stats.min_parser_ms
                        }
                    ))
                    .weak(),
                );
                ui.label(egui::RichText::new(format!("{:.3}ms", stats.max_parser_ms)).weak());
                ui.end_row();

                ui.label("UI Sync:");
                ui.label(
                    egui::RichText::new(format!("{ui_latency:.3}ms")).color(egui::Color32::GOLD),
                );
                ui.label(
                    egui::RichText::new(format!("{avg_ui_latency:.3}ms"))
                        .color(egui::Color32::GOLD)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}ms",
                        if min_ui_latency == f32::MAX {
                            0.0
                        } else {
                            min_ui_latency
                        }
                    ))
                    .weak(),
                );
                ui.label(egui::RichText::new(format!("{max_ui_latency:.3}ms")).weak());
                ui.end_row();

                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.end_row();

                let total_current = stats.hid_read_ms + stats.parser_ms + ui_latency;
                ui.label(egui::RichText::new("Total Software Lag:").strong());
                ui.label(
                    egui::RichText::new(format!("{total_current:.3}ms"))
                        .strong()
                        .color(egui::Color32::WHITE),
                );
                ui.end_row();
            });

        ui.add_space(5.0);
        ui.weak("Total Software Lag: USB Arrival -> UI Paint. Excludes OS/Monitor lag.");
    });

    ui.add_space(20.0);

    ui.columns(2, |cols| {
        if let [col0, col1, ..] = cols {
            col0.group(|ui| {
                ui.label(egui::RichText::new("Packet Flow").strong());
                ui.add_space(5.0);
                ui.label(format!("Total Count: {}", stats.total_packets));
                ui.label(format!("Polling Rate: {displayed_hz:.1} Hz"));

                if displayed_hz > 1.0 {
                    let interval = 1000.0 / displayed_hz;
                    ui.label(format!("Avg Interval: {interval:.2} ms"));
                } else {
                    ui.label("Avg Interval: Static / Idle");
                }
            });

            col1.group(|ui| {
                ui.label(egui::RichText::new("Hardware Info").strong());
                ui.add_space(5.0);
                ui.label(format!(
                    "Resolution: {} x {}",
                    (max_w.max(0.0)) as u32,
                    (max_h.max(0.0)) as u32
                ));
                ui.label(format!("Current Pen Status: {}", tablet_data.status));
                ui.label(format!(
                    "Connected: {}",
                    if tablet_data.is_connected {
                        "Yes"
                    } else {
                        "No"
                    }
                ));
            });
        }
    });

    ui.add_space(20.0);

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new("Live Packet Capture (Real-Time)").strong());
        ui.add_space(8.0);

        egui::Frame::new()
            .fill(egui::Color32::from_gray(20))
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("COORDS").weak().size(9.0));
                        ui.label(format!("X: {:<5}", tablet_data.x));
                        ui.label(format!("Y: {:<5}", tablet_data.y));
                    });
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("PRESSURE").weak().size(9.0));
                        ui.label(format!("{:<5}", tablet_data.pressure));
                    });
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("TILT").weak().size(9.0));
                        ui.label(format!("X: {:<3}", tablet_data.tilt_x));
                        ui.label(format!("Y: {:<3}", tablet_data.tilt_y));
                    });
                });

                ui.add_space(10.0);
                ui.label(egui::RichText::new("RAW BYTES").weak().size(9.0));
                ui.label(
                    egui::RichText::new(&tablet_data.raw_data)
                        .code()
                        .size(11.0)
                        .color(egui::Color32::LIGHT_GRAY),
                );
            });
    });

    reset_requested
}
