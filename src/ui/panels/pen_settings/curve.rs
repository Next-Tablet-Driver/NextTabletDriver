//! Pressure response curve card.
//!
//! Curve-type selector, exponent input, a hand-drawn draggable-point editor
//! for the `Custom` curve type, and a live pressure meter showing raw vs.
//! curve-shaped pressure side by side.

use crate::app::state::UiSnapshot;
use crate::core::config::models::{MappingConfig, PressureCurveType};
use crate::core::math::curve::evaluate_custom;
use crate::t;
use crate::ui::theme::{accent_bg, panel_bg, panel_border, ui_card, ui_setting_row_range};
use eframe::egui;

const MAX_POINTS: usize = 32;
const POINT_HIT_RADIUS: f32 = 10.0;

pub fn render_pressure_curve_card(
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    snapshot: &UiSnapshot,
) {
    ui_card(
        ui,
        &t!("pen.pressure_curve.title"),
        egui_phosphor::regular::CHART_LINE,
        |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut config.pressure_curve.curve_type,
                    PressureCurveType::Linear,
                    t!("pen.pressure_curve.linear"),
                );
                ui.selectable_value(
                    &mut config.pressure_curve.curve_type,
                    PressureCurveType::Exponential,
                    t!("pen.pressure_curve.exponential"),
                );
                ui.selectable_value(
                    &mut config.pressure_curve.curve_type,
                    PressureCurveType::Custom,
                    t!("pen.pressure_curve.custom"),
                );
            });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // Pin the left column's width so it claims the same
                    // horizontal space back from this row in every curve
                    // mode; otherwise the meter's position would shift (or
                    // get pushed out of view) depending on how wide each
                    // match arm's content happens to be.
                    ui.set_width(CANVAS_SIZE);
                    render_curve_editor(ui, config);
                });

                ui.add_space(10.0);
                render_pressure_meter(ui, config, snapshot);
            });
        },
    );
}

const CANVAS_SIZE: f32 = 220.0;

fn render_curve_editor(ui: &mut egui::Ui, config: &mut MappingConfig) {
    match config.pressure_curve.curve_type {
        PressureCurveType::Linear => {}
        PressureCurveType::Exponential => {
            ui_setting_row_range(
                ui,
                &t!("pen.pressure_curve.exponent"),
                &mut config.pressure_curve.exponent,
                "",
                0.1..=5.0,
            );
        }
        PressureCurveType::Custom => {
            render_curve_canvas(ui, config);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(t!("pen.pressure_curve.canvas_help"))
                        .weak()
                        .size(10.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(t!("pen.pressure_curve.reset")).clicked() {
                        config.pressure_curve.points = vec![(0.0, 0.0), (1.0, 1.0)];
                    }
                });
            });
        }
    }
}

fn render_curve_canvas(ui: &mut egui::Ui, config: &mut MappingConfig) {
    egui::Frame::canvas(ui.style())
        .fill(panel_bg(ui.visuals()))
        .stroke(egui::Stroke::new(1.0_f32, panel_border(ui.visuals())))
        .show(ui, |ui| {
            let (rect, response) = ui.allocate_at_least(
                egui::vec2(CANVAS_SIZE, CANVAS_SIZE),
                egui::Sense::click_and_drag(),
            );

            let to_screen = |t: f32, y: f32| -> egui::Pos2 {
                egui::pos2(
                    t.mul_add(rect.width(), rect.left()),
                    y.mul_add(-rect.height(), rect.bottom()),
                )
            };
            let from_screen = |pos: egui::Pos2| -> (f32, f32) {
                (
                    ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0),
                    ((rect.bottom() - pos.y) / rect.height()).clamp(0.0, 1.0),
                )
            };

            // Faint diagonal reference line (the identity curve).
            let weak_color = ui.visuals().weak_text_color();
            ui.painter().line_segment(
                [to_screen(0.0, 0.0), to_screen(1.0, 1.0)],
                egui::Stroke::new(1.0_f32, weak_color.gamma_multiply(0.5)),
            );

            let curve_color = ui.visuals().strong_text_color();
            let samples: u32 = 64;
            let curve_points: Vec<egui::Pos2> = (0..=samples)
                .map(|i| {
                    let t = i as f32 / samples as f32;
                    let y = evaluate_custom(t, &config.pressure_curve.points);
                    to_screen(t, y)
                })
                .collect();
            ui.painter().add(egui::Shape::line(
                curve_points,
                egui::Stroke::new(2.0_f32, curve_color),
            ));

            handle_curve_interaction(ui, &response, to_screen, from_screen, config);

            let accent = accent_bg(ui.visuals());
            for &(px, py) in &config.pressure_curve.points {
                let center = to_screen(px, py);
                ui.painter().circle_filled(center, 5.0, accent);
                ui.painter()
                    .circle_stroke(center, 5.0, egui::Stroke::new(1.5_f32, curve_color));
            }
        });
}

const METER_WIDTH: f32 = 96.0;
const METER_BAR_WIDTH: f32 = 24.0;
const METER_BAR_GAP: f32 = 18.0;
const METER_LABEL_MARGIN: f32 = 18.0;

/// Live pressure meter next to the curve editor: two vertical bars showing
/// the tablet's raw pressure alongside the same value reshaped by the curve
/// currently being edited, so its effect can be read at a glance instead of
/// only off the graph.
fn render_pressure_meter(ui: &mut egui::Ui, config: &MappingConfig, snapshot: &UiSnapshot) {
    let tablet_data = &snapshot.tablet_data;
    let max_p = 8192.0_f32;
    let raw_ratio = if tablet_data.is_connected {
        (f32::from(tablet_data.pressure) / max_p).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let curved_ratio = crate::core::math::curve::evaluate(raw_ratio, &config.pressure_curve);

    let raw_label = t!("pen.pressure_curve.meter_raw");
    let curved_label = t!("pen.pressure_curve.meter_curved");

    egui::Frame::canvas(ui.style())
        .fill(panel_bg(ui.visuals()))
        .stroke(egui::Stroke::new(1.0_f32, panel_border(ui.visuals())))
        .show(ui, |ui| {
            let (rect, _) =
                ui.allocate_at_least(egui::vec2(METER_WIDTH, CANVAS_SIZE), egui::Sense::hover());

            let track_top = rect.top() + METER_LABEL_MARGIN + 4.0;
            let track_bottom = rect.bottom() - METER_LABEL_MARGIN - 4.0;
            let bars_total_w = METER_BAR_WIDTH.mul_add(2.0, METER_BAR_GAP);
            let start_x = rect.center().x - bars_total_w / 2.0;

            let weak = ui.visuals().weak_text_color();
            let accent = accent_bg(ui.visuals());
            let text_color = ui.visuals().text_color();
            let track_bg = ui.visuals().extreme_bg_color;
            let border = panel_border(ui.visuals());

            for (i, (ratio, color, label)) in [
                (raw_ratio, weak, raw_label.as_str()),
                (curved_ratio, accent, curved_label.as_str()),
            ]
            .into_iter()
            .enumerate()
            {
                let x = (i as f32).mul_add(METER_BAR_WIDTH + METER_BAR_GAP, start_x);

                let track = egui::Rect::from_min_max(
                    egui::pos2(x, track_top),
                    egui::pos2(x + METER_BAR_WIDTH, track_bottom),
                );
                ui.painter().rect_filled(track, 4.0, track_bg);

                let fill_height = track.height() * ratio;
                if fill_height > 0.5 {
                    let fill = egui::Rect::from_min_max(
                        egui::pos2(x, track.bottom() - fill_height),
                        egui::pos2(x + METER_BAR_WIDTH, track.bottom()),
                    );
                    ui.painter().rect_filled(fill, 4.0, color);
                }
                ui.painter().rect_stroke(
                    track,
                    4.0,
                    egui::Stroke::new(1.0_f32, border),
                    egui::StrokeKind::Middle,
                );

                ui.painter().text(
                    egui::pos2(x + METER_BAR_WIDTH / 2.0, rect.top() + 8.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.0}%", ratio * 100.0),
                    egui::FontId::monospace(11.0),
                    text_color,
                );
                ui.painter().text(
                    egui::pos2(x + METER_BAR_WIDTH / 2.0, rect.bottom() - 8.0),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(9.0),
                    weak,
                );
            }
        });
}

fn nearest_point_index(
    pointer_pos: egui::Pos2,
    points: &[(f32, f32)],
    to_screen: impl Fn(f32, f32) -> egui::Pos2,
) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, &(px, py))| (i, to_screen(px, py).distance(pointer_pos)))
        .filter(|&(_, dist)| dist <= POINT_HIT_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

fn handle_curve_interaction(
    ui: &egui::Ui,
    response: &egui::Response,
    to_screen: impl Fn(f32, f32) -> egui::Pos2,
    from_screen: impl Fn(egui::Pos2) -> (f32, f32),
    config: &mut MappingConfig,
) {
    let drag_id = ui.id().with("pressure_curve_drag");

    if response.drag_started()
        && let Some(pointer_pos) = response.interact_pointer_pos()
    {
        let hit = nearest_point_index(pointer_pos, &config.pressure_curve.points, &to_screen);
        ui.data_mut(|d| d.insert_temp(drag_id, hit));
    }

    if response.dragged()
        && let Some(pointer_pos) = response.interact_pointer_pos()
    {
        let dragged_index = ui.data_mut(|d| d.get_temp::<Option<usize>>(drag_id).flatten());
        if let Some(idx) = dragged_index {
            let (raw_t, raw_y) = from_screen(pointer_pos);
            let points = &mut config.pressure_curve.points;
            let last_idx = points.len().saturating_sub(1);

            let prev_x = idx
                .checked_sub(1)
                .and_then(|i| points.get(i))
                .map_or(0.0, |p| p.0);
            let next_x = points.get(idx + 1).map_or(1.0, |p| p.0);

            if let Some(point) = points.get_mut(idx) {
                point.1 = raw_y.clamp(0.0, 1.0);
                if idx == 0 {
                    point.0 = 0.0;
                } else if idx == last_idx {
                    point.0 = 1.0;
                } else {
                    const EPS: f32 = 0.001;
                    point.0 = raw_t.clamp(prev_x + EPS, next_x - EPS);
                }
            }
        }
    }

    if response.drag_stopped() {
        ui.data_mut(|d| d.insert_temp(drag_id, Option::<usize>::None));
    }

    if response.double_clicked()
        && let Some(pointer_pos) = response.interact_pointer_pos()
    {
        let (t, y) = from_screen(pointer_pos);
        insert_point(&mut config.pressure_curve.points, t, y);
    }

    if response.secondary_clicked()
        && let Some(pointer_pos) = response.interact_pointer_pos()
    {
        let points = &config.pressure_curve.points;
        let last_idx = points.len().saturating_sub(1);
        if let Some(idx) = nearest_point_index(pointer_pos, points, &to_screen)
            && idx != 0
            && idx != last_idx
        {
            config.pressure_curve.points.remove(idx);
        }
    }
}

fn insert_point(points: &mut Vec<(f32, f32)>, t: f32, y: f32) {
    if points.len() >= MAX_POINTS {
        return;
    }
    let t = t.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    let idx = points.partition_point(|p| p.0 < t);
    points.insert(idx, (t, y));
}
