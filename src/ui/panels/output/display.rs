use crate::app::state::TabletMapperApp;
use crate::core::config::models::MappingConfig;
use crate::t;
use crate::ui::theme::{ui_input_box, ui_section_header};
use eframe::egui;

pub fn render_display_section(
    app: &TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) {
    ui_section_header(ui, &t!("output.display"));

    egui::Frame::canvas(ui.style())
        .fill(crate::ui::theme::panel_bg(ui.visuals()))
        .stroke(egui::Stroke::new(
            1.0_f32,
            crate::ui::theme::panel_border(ui.visuals()),
        ))
        .show(ui, |ui| {
            let available_w = ui.available_width();
            let viz_h = 200.0;
            let (rect, response) = ui.allocate_at_least(
                egui::vec2(available_w, viz_h),
                egui::Sense::click_and_drag(),
            );

            let desk_w = max_x - min_x;
            let desk_h = max_y - min_y;

            if desk_w > 0.0 {
                let scale = (rect.width() / desk_w).min(rect.height() / desk_h) * 0.9;
                let offset_x = rect.center().x - (desk_w * scale) / 2.0;
                let offset_y = rect.center().y - (desk_h * scale) / 2.0;

                for d in &app.displays {
                    let s_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            (d.x as f32 - min_x).mul_add(scale, offset_x),
                            (d.y as f32 - min_y).mul_add(scale, offset_y),
                        ),
                        egui::vec2(d.width as f32 * scale, d.height as f32 * scale),
                    );

                    ui.painter().rect_stroke(
                        s_rect,
                        0.0,
                        egui::Stroke::new(1.0_f32, crate::ui::theme::panel_border(ui.visuals())),
                        egui::StrokeKind::Middle,
                    );
                    ui.painter().text(
                        s_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{}px", d.width),
                        egui::FontId::proportional(10.0),
                        ui.visuals().text_color(),
                    );
                }

                let t_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        (config.target_area.x - min_x).mul_add(scale, offset_x),
                        (config.target_area.y - min_y).mul_add(scale, offset_y),
                    ),
                    egui::vec2(config.target_area.w * scale, config.target_area.h * scale),
                );
                let stroke_color = ui.visuals().strong_text_color();

                ui.painter()
                    .rect_filled(t_rect, 0.0, crate::ui::theme::accent_bg(ui.visuals()));
                ui.painter().rect_stroke(
                    t_rect,
                    0.0,
                    egui::Stroke::new(1.0_f32, stroke_color),
                    egui::StrokeKind::Middle,
                );

                ui.painter()
                    .circle_filled(t_rect.center(), 1.5, stroke_color);

                let font_id = egui::FontId::proportional(12.0);
                let color = ui.visuals().window_fill;

                ui.painter().text(
                    t_rect.center_top() + egui::vec2(0.0, 5.0),
                    egui::Align2::CENTER_TOP,
                    format!("{}px", config.target_area.w as i32),
                    font_id.clone(),
                    color,
                );

                let left_mid = t_rect.left_center();
                let galley = ui.fonts_mut(|f| {
                    f.layout_no_wrap(
                        format!("{}px", config.target_area.h as i32),
                        font_id.clone(),
                        color,
                    )
                });
                ui.painter().add(egui::epaint::TextShape {
                    pos: left_mid + egui::vec2(5.0, 0.0),
                    galley,
                    underline: egui::Stroke::NONE,
                    override_text_color: None,
                    angle: -std::f32::consts::FRAC_PI_2,
                    fallback_color: color,
                    opacity_factor: 1.0,
                });

                let ratio = if config.target_area.h == 0.0 {
                    0.0
                } else {
                    config.target_area.w / config.target_area.h
                };
                ui.painter().text(
                    t_rect.center() + egui::vec2(0.0, 12.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{ratio:.4}").replace('.', ","),
                    font_id,
                    color,
                );

                let drag_id = ui.id().with("display_drag_raw_pos");
                let mut raw_pos = ui.data_mut(|d| {
                    d.get_temp::<(f32, f32)>(drag_id)
                        .unwrap_or((config.target_area.x, config.target_area.y))
                });

                if response.drag_started() {
                    raw_pos = (config.target_area.x, config.target_area.y);
                    ui.data_mut(|d| d.insert_temp(drag_id, raw_pos));
                }

                if response.dragged()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                {
                    if t_rect.expand(20.0).contains(pointer_pos) || response.drag_started() {
                        raw_pos.0 += response.drag_delta().x / scale;
                        raw_pos.1 += response.drag_delta().y / scale;
                        ui.data_mut(|d| d.insert_temp(drag_id, raw_pos));

                        let mut new_x = raw_pos.0;
                        let mut new_y = raw_pos.1;

                        if config.display_snapping {
                            let snap_dist = 120.0; // 120px
                            let mut best_x = new_x;
                            let mut min_dist_x = f32::MAX;

                            for edge_x in app
                                .displays
                                .iter()
                                .flat_map(|d| [d.x as f32, d.x as f32 + d.width as f32])
                            {
                                let dist_left = (new_x - edge_x).abs();
                                if dist_left < snap_dist && dist_left < min_dist_x {
                                    min_dist_x = dist_left;
                                    best_x = edge_x;
                                }
                                let dist_right = ((new_x + config.target_area.w) - edge_x).abs();
                                if dist_right < snap_dist && dist_right < min_dist_x {
                                    min_dist_x = dist_right;
                                    best_x = edge_x - config.target_area.w;
                                }
                            }

                            let mut best_y = new_y;
                            let mut min_dist_y = f32::MAX;

                            for edge_y in app
                                .displays
                                .iter()
                                .flat_map(|d| [d.y as f32, d.y as f32 + d.height as f32])
                            {
                                let dist_top = (new_y - edge_y).abs();
                                if dist_top < snap_dist && dist_top < min_dist_y {
                                    min_dist_y = dist_top;
                                    best_y = edge_y;
                                }
                                let dist_bottom = ((new_y + config.target_area.h) - edge_y).abs();
                                if dist_bottom < snap_dist && dist_bottom < min_dist_y {
                                    min_dist_y = dist_bottom;
                                    best_y = edge_y - config.target_area.h;
                                }
                            }

                            new_x = best_x;
                            new_y = best_y;
                        }

                        config.target_area.x = new_x.clamp(min_x, max_x - config.target_area.w);
                        config.target_area.y = new_y.clamp(min_y, max_y - config.target_area.h);
                    }
                } else if response.drag_stopped() {
                    ui.data_mut(|d| d.remove::<egui::Vec2>(drag_id));
                }
            }
        });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.checkbox(&mut config.display_snapping, t!("output.edge_snapping"));
    });

    ui.horizontal(|ui| {
        ui.add_space(20.0);
        egui::Grid::new("display_grid")
            .spacing(egui::vec2(10.0, 10.0))
            .show(ui, |ui| {
                ui_input_box(ui, "Width", &mut config.target_area.w, "px");
                ui_input_box(ui, "Height", &mut config.target_area.h, "px");

                config.target_area.w = config.target_area.w.clamp(10.0, max_x - min_x);
                config.target_area.h = config.target_area.h.clamp(10.0, max_y - min_y);

                let mut ui_x = config.target_area.x - min_x + config.target_area.w / 2.0;
                let mut ui_y = config.target_area.y - min_y + config.target_area.h / 2.0;

                let old_ui_x = ui_x;
                let old_ui_y = ui_y;

                ui_input_box(ui, "X", &mut ui_x, "px");
                ui_input_box(ui, "Y", &mut ui_y, "px");

                if (ui_x - old_ui_x).abs() > 0.1 {
                    config.target_area.x = (ui_x - config.target_area.w / 2.0 + min_x)
                        .clamp(min_x, max_x - config.target_area.w);
                }
                if (ui_y - old_ui_y).abs() > 0.1 {
                    config.target_area.y = (ui_y - config.target_area.h / 2.0 + min_y)
                        .clamp(min_y, max_y - config.target_area.h);
                }
                ui.end_row();
            });
    });
}
