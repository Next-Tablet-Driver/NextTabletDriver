use crate::app::state::TabletMapperApp;
use crate::core::config::models::MappingConfig;
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
    ui_section_header(ui, "Display");

    egui::Frame::canvas(ui.style())
        .fill(crate::ui::theme::panel_bg(ui.visuals()))
        .stroke(egui::Stroke::new(
            1.0,
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
                        egui::Stroke::new(1.0, crate::ui::theme::panel_border(ui.visuals())),
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
                    egui::Stroke::new(1.0, stroke_color),
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

                let drag_id = ui.id().with("display_drag");
                let mut current_drag_delta =
                    ui.data_mut(|d| d.get_temp::<egui::Vec2>(drag_id).unwrap_or_default());

                if response.dragged()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                {
                    if t_rect.expand(20.0).contains(pointer_pos) || response.drag_started() {
                        current_drag_delta += response.drag_delta() / scale;
                        ui.data_mut(|d| d.insert_temp(drag_id, current_drag_delta));

                        config.target_area.x = (config.target_area.x
                            + response.drag_delta().x / scale)
                            .clamp(min_x, max_x - config.target_area.w);
                        config.target_area.y = (config.target_area.y
                            + response.drag_delta().y / scale)
                            .clamp(min_y, max_y - config.target_area.h);
                    }
                } else if response.drag_stopped() {
                    ui.data_mut(|d| d.insert_temp(drag_id, egui::Vec2::ZERO));
                }
            }
        });

    ui.add_space(8.0);

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
