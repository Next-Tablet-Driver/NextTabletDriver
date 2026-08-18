use crate::app::state::UiSnapshot;
use crate::engine::state::SharedState;
use crate::t;
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
    shared: &Arc<SharedState>,
) -> bool {
    let tablet_data = &snapshot.tablet_data;
    let stats = &snapshot.stats;
    let (max_w, max_h) = snapshot.hardware_size;
    let mut reset_requested = false;
    let semantic = crate::ui::theme::semantic_colors(ui.ctx());

    ui.add_space(10.0);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(t!("performance.latency_title"))
                    .strong()
                    .size(14.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(t!("performance.reset_stats")).clicked() {
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
                ui.label(t!("performance.component"));
                ui.label(t!("performance.current"));
                ui.label(t!("performance.avg_ema"));
                ui.label(t!("performance.min"));
                ui.label(t!("performance.max"));
                ui.end_row();

                ui.label(t!("performance.hid_read"));
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.hid_read_ms)).color(semantic.info),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.avg_hid_read_ms))
                        .color(semantic.info)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}ms",
                        if (stats.min_hid_read_ms - f32::MAX).abs() < f32::EPSILON {
                            0.0
                        } else {
                            stats.min_hid_read_ms
                        }
                    ))
                    .weak(),
                );
                ui.label(egui::RichText::new(format!("{:.3}ms", stats.max_hid_read_ms)).weak());
                ui.end_row();

                ui.label(t!("performance.parser"));
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.parser_ms))
                        .color(semantic.success),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.avg_parser_ms))
                        .color(semantic.success)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}ms",
                        if (stats.min_parser_ms - f32::MAX).abs() < f32::EPSILON {
                            0.0
                        } else {
                            stats.min_parser_ms
                        }
                    ))
                    .weak(),
                );
                ui.label(egui::RichText::new(format!("{:.3}ms", stats.max_parser_ms)).weak());
                ui.end_row();

                ui.label(t!("performance.inject"));
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.inject_ms))
                        .color(semantic.success),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}ms", stats.avg_inject_ms))
                        .color(semantic.success)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}ms",
                        if (stats.min_inject_ms - f32::MAX).abs() < f32::EPSILON {
                            0.0
                        } else {
                            stats.min_inject_ms
                        }
                    ))
                    .weak(),
                );
                ui.label(egui::RichText::new(format!("{:.3}ms", stats.max_inject_ms)).weak());
                ui.end_row();

                ui.label(t!("performance.ui_sync"));
                ui.label(egui::RichText::new(format!("{ui_latency:.3}ms")).color(semantic.warning));
                ui.label(
                    egui::RichText::new(format!("{avg_ui_latency:.3}ms"))
                        .color(semantic.warning)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}ms",
                        if (min_ui_latency - f32::MAX).abs() < f32::EPSILON {
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

                let total_current =
                    stats.hid_read_ms + stats.parser_ms + stats.inject_ms + ui_latency;
                ui.label(egui::RichText::new(t!("performance.total_lag")).strong());
                ui.label(
                    egui::RichText::new(format!("{total_current:.3}ms"))
                        .strong()
                        .color(ui.visuals().strong_text_color()),
                );
                ui.end_row();
            });

        ui.add_space(5.0);
        ui.weak(t!("performance.total_lag_note"));
    });

    ui.add_space(20.0);

    ui.columns(2, |cols| {
        if let [col0, col1, ..] = cols {
            col0.group(|ui| {
                ui.label(egui::RichText::new(t!("performance.packet_flow")).strong());
                ui.add_space(5.0);
                ui.label(t!("performance.total_count", count = stats.total_packets));
                ui.label(t!(
                    "performance.polling_rate",
                    hz = format!("{displayed_hz:.1}")
                ));

                if displayed_hz > 1.0 {
                    let interval = 1000.0 / displayed_hz;
                    ui.label(t!(
                        "performance.avg_interval",
                        interval = format!("{interval:.2}")
                    ));
                } else {
                    ui.label(t!("performance.idle"));
                }
            });

            col1.group(|ui| {
                ui.label(egui::RichText::new(t!("performance.hardware_info")).strong());
                ui.add_space(5.0);
                ui.label(t!(
                    "performance.resolution",
                    w = (max_w.max(0.0)) as u32,
                    h = (max_h.max(0.0)) as u32
                ));
                ui.label(t!(
                    "performance.pen_status",
                    status = tablet_data.status.as_str()
                ));
                if tablet_data.is_connected {
                    ui.label(t!("performance.connected_yes"));
                } else {
                    ui.label(t!("performance.connected_no"));
                }
            });
        }
    });

    ui.add_space(20.0);

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(t!("performance.live_capture")).strong());
        ui.add_space(8.0);

        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
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
                    egui::RichText::new(tablet_data.raw_hex())
                        .code()
                        .size(11.0)
                        .color(ui.visuals().text_color()),
                );
            });
    });

    reset_requested
}
