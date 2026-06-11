use crate::app::state::UiSnapshot;
use crate::core::config::models::MappingConfig;
use crate::core::math::geometry::ActiveAreaGeometry;
use crate::ui::theme::{ui_input_box, ui_section_header};
use eframe::egui;

pub fn render_tablet_section(ui: &mut egui::Ui, config: &mut MappingConfig, snapshot: &UiSnapshot) {
    ui_section_header(ui, "Tablet");

    if let crate::engine::state::EngineStatus::Failed(ref reason) = snapshot.engine_status {
        ui.add(egui::Label::new(
            egui::RichText::new(format!("Engine Failed: {reason}"))
                .color(ui.visuals().error_fg_color)
                .heading(),
        ));
        ui.add_space(10.0);
    }

    let (phys_w, phys_h) = snapshot.physical_size;
    let tablet_data = &snapshot.tablet_data;

    egui::Frame::canvas(ui.style())
        .fill(crate::ui::theme::panel_bg(ui.visuals()))
        .stroke(egui::Stroke::new(
            1.0,
            crate::ui::theme::panel_border(ui.visuals()),
        ))
        .show(ui, |ui| {
            let available_w = ui.available_width();
            let viz_h = 250.0;
            let (rect, response) = ui.allocate_at_least(
                egui::vec2(available_w, viz_h),
                egui::Sense::click_and_drag(),
            );

            let geo = ActiveAreaGeometry::calculate(
                phys_w,
                phys_h,
                rect.width(),
                rect.height(),
                rect.center().x,
                rect.center().y,
                &config.active_area,
                config.target_area.w,
                config.target_area.h,
                config.show_osu_playfield,
            );

            let full_rect = egui::Rect::from_min_size(
                egui::pos2(geo.offset_x, geo.offset_y),
                egui::vec2(phys_w * geo.scale, phys_h * geo.scale),
            );

            ui.painter().rect_stroke(
                full_rect,
                0.0,
                egui::Stroke::new(1.0, crate::ui::theme::panel_border(ui.visuals())),
                egui::StrokeKind::Middle,
            );

            let points: Vec<egui::Pos2> =
                geo.points.iter().map(|(x, y)| egui::pos2(*x, *y)).collect();

            let stroke_color = ui.visuals().strong_text_color();

            ui.painter().add(egui::Shape::convex_polygon(
                points.clone(),
                crate::ui::theme::accent_bg(ui.visuals()),
                egui::Stroke::new(1.0, stroke_color),
            ));

            ui.painter().circle_filled(
                egui::pos2(geo.aa_center_x, geo.aa_center_y),
                1.5,
                stroke_color,
            );

            if let Some(pf_points) = geo.osu_playfield_points {
                let pf_points: Vec<egui::Pos2> =
                    pf_points.iter().map(|(x, y)| egui::pos2(*x, *y)).collect();

                let semantic = crate::ui::theme::semantic_colors(ui.ctx());
                ui.painter().add(egui::Shape::convex_polygon(
                    pf_points,
                    semantic
                        .playfield
                        .gamma_multiply(semantic.playfield_opacity),
                    egui::Stroke::new(1.5, semantic.playfield),
                ));
            }

            let font_id = egui::FontId::proportional(11.0);
            let color = ui.visuals().window_fill;

            let rot_rad = config.active_area.rotation.to_radians();
            if let [p0, p1, _, p3, ..] = points.as_slice() {
                let left_mid = egui::pos2(f32::midpoint(p0.x, p3.x), f32::midpoint(p0.y, p3.y));
                let galley = ui.fonts_mut(|f| {
                    let text = format!("{:.2}", config.active_area.h)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .replace('.', ",")
                        + "mm";
                    f.layout_no_wrap(text, font_id.clone(), color)
                });
                ui.painter().add(egui::epaint::TextShape {
                    pos: left_mid
                        + egui::vec2(
                            5.0f32.mul_add(rot_rad.cos(), -(2.0 * rot_rad.sin())),
                            5.0f32.mul_add(rot_rad.sin(), 2.0 * rot_rad.cos()),
                        ),
                    galley,
                    underline: egui::Stroke::NONE,
                    override_text_color: None,
                    angle: -std::f32::consts::FRAC_PI_2 + rot_rad,
                    fallback_color: color,
                    opacity_factor: 1.0,
                });

                let ratio = if config.active_area.h == 0.0 {
                    0.0
                } else {
                    config.active_area.w / config.active_area.h
                };

                ui.painter().text(
                    egui::pos2(geo.aa_center_x, geo.aa_center_y + 12.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{ratio:.4}").replace('.', ","),
                    font_id.clone(),
                    color,
                );

                let top_mid = egui::pos2(f32::midpoint(p0.x, p1.x), f32::midpoint(p0.y, p1.y));
                let text_w = format!("{:.2}", config.active_area.w)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .replace('.', ",")
                    + "mm";
                ui.painter().text(
                    top_mid + egui::vec2(5.0 * rot_rad.sin(), 5.0 * rot_rad.cos()),
                    egui::Align2::CENTER_TOP,
                    text_w,
                    font_id,
                    color,
                );
            }

            handle_tablet_drag(ui, &response, &points, geo.scale, phys_w, phys_h, config);

            if tablet_data.is_connected {
                let (max_w, max_h) = snapshot.hardware_size;
                let cx =
                    ((f32::from(tablet_data.x) / max_w) * phys_w).mul_add(geo.scale, geo.offset_x);
                let cy =
                    ((f32::from(tablet_data.y) / max_h) * phys_h).mul_add(geo.scale, geo.offset_y);
                if full_rect.contains(egui::pos2(cx, cy)) {
                    let cursor_color = ui.visuals().strong_text_color();
                    ui.painter()
                        .circle_filled(egui::pos2(cx, cy), 3.0, cursor_color);
                }
            }
        });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut config.lock_aspect_ratio, "Force Aspect Ratio");
                ui.add_space(10.0);
                ui.checkbox(&mut config.show_osu_playfield, "Show Osu!Playfield");
            });
            egui::Grid::new("tablet_grid")
                .spacing(egui::vec2(10.0, 10.0))
                .show(ui, |ui| {
                    let mut w = config.active_area.w;
                    let mut h = config.active_area.h;

                    ui_input_box(ui, "Width", &mut w, "mm");
                    if (w - config.active_area.w).abs() > f32::EPSILON {
                        if config.lock_aspect_ratio && config.active_area.h > 0.0 {
                            let ratio = config.active_area.w / config.active_area.h;
                            config.active_area.w = w;
                            config
                                .active_area
                                .apply_aspect_ratio(ratio, true, phys_w, phys_h);
                        } else {
                            config.active_area.w = w;
                        }
                    }

                    ui_input_box(ui, "Height", &mut h, "mm");
                    if (h - config.active_area.h).abs() > f32::EPSILON {
                        if config.lock_aspect_ratio && config.active_area.h > 0.0 {
                            let ratio = config.active_area.w / config.active_area.h;
                            config.active_area.h = h;
                            config
                                .active_area
                                .apply_aspect_ratio(ratio, false, phys_w, phys_h);
                        } else {
                            config.active_area.h = h;
                        }
                    }

                    ui_input_box(ui, "X", &mut config.active_area.x, "mm");
                    ui_input_box(ui, "Y", &mut config.active_area.y, "mm");
                    ui_input_box(ui, "Rotation", &mut config.active_area.rotation, "°");

                    ui.end_row();

                    config.active_area.clamp_to_surface(phys_w, phys_h);
                });
        });
    });
}

fn handle_tablet_drag(
    ui: &egui::Ui,
    response: &egui::Response,
    points: &[egui::Pos2],
    scale: f32,
    phys_w: f32,
    phys_h: f32,
    config: &mut MappingConfig,
) {
    let drag_id = ui.id().with("tablet_drag");
    let mut current_drag_delta =
        ui.data_mut(|d| d.get_temp::<egui::Vec2>(drag_id).unwrap_or_default());

    if response.dragged() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let click_rect = egui::Rect::from_min_max(
                points
                    .iter()
                    .fold(egui::pos2(f32::INFINITY, f32::INFINITY), |a, b| {
                        egui::pos2(a.x.min(b.x), a.y.min(b.y))
                    }),
                points
                    .iter()
                    .fold(egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY), |a, b| {
                        egui::pos2(a.x.max(b.x), a.y.max(b.y))
                    }),
            );

            if click_rect.expand(20.0).contains(pointer_pos) || response.drag_started() {
                let delta = response.drag_delta() / scale;
                current_drag_delta += delta;
                ui.data_mut(|d| d.insert_temp(drag_id, current_drag_delta));

                config.active_area.x += delta.x;
                config.active_area.y += delta.y;
                config.active_area.clamp_to_surface(phys_w, phys_h);
            }
        }
    } else if response.drag_stopped() {
        ui.data_mut(|d| d.insert_temp(drag_id, egui::Vec2::ZERO));
    }
}
