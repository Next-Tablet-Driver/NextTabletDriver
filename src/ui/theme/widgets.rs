//! Reusable styled egui widget helpers built on top of the semantic palette.

use super::colors::{label_color, panel_bg, panel_border};

use eframe::egui;

/// Renders a standardized section header with a title and a horizontal separator line.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `title` - The header text to display.
pub fn ui_section_header(ui: &mut egui::Ui, title: &str) {
    let text_color = ui.visuals().strong_text_color();
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(egui::RichText::new(title).size(16.0).color(text_color));
    });
    ui.add(egui::Separator::default().spacing(4.0).grow(2.0));
}

/// Core helper for creating a styled, labeled card/box container grouping.
///
/// Returns the value produced by the contents closure.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - A string slice containing the label text.
/// * `width` - The width of the container box in pixels.
/// * `add_contents` - A closure that defines the contents inside the box.
pub fn ui_labeled_box<R>(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let visuals = ui.visuals();
    let bg_fill = panel_bg(visuals);
    let border_color = panel_border(visuals);
    let label_clr = label_color(visuals);

    ui.scope(|ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(6.0, 2.0);

        egui::Frame::new()
            .fill(bg_fill)
            .corner_radius(4.0)
            .stroke(egui::Stroke::new(1.0_f32, border_color.gamma_multiply(0.6)))
            .inner_margin(egui::Margin::symmetric(10, 5))
            .show(ui, |ui| {
                ui.set_width(width);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(label)
                            .size(11.0)
                            .color(label_clr)
                            .strong(),
                    );

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        add_contents,
                    )
                    .inner
                })
                .inner
            })
            .inner
    })
    .inner
}

/// Renders a styled container holding a label and an `f32` `DragValue` input with an inclusive range.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the value.
/// * `value` - A mutable reference to the `f32` value to edit.
/// * `unit` - A suffix indicating the unit of measurement (e.g., "mm", "px").
/// * `range` - The inclusive range of valid values.
pub fn ui_input_box_range(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    unit: &str,
    range: std::ops::RangeInclusive<f32>,
) {
    ui_labeled_box(ui, label, 140.0, |ui| {
        let label_clr = label_color(ui.visuals());
        if !unit.is_empty() {
            ui.label(
                egui::RichText::new(unit)
                    .size(10.0)
                    .color(label_clr.gamma_multiply(0.5)),
            );
            ui.add_space(2.0);
        }

        let response = ui.add(
            egui::DragValue::new(value)
                .speed(0.1)
                .max_decimals(2)
                .range(range)
                .custom_formatter(|val, _| {
                    format!("{val:.2}")
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .replace('.', ",")
                }),
        );
        if response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }
    });
}

/// Renders a styled container holding a label and an unbounded `f32` `DragValue` input.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the value.
/// * `value` - A mutable reference to the `f32` value to edit.
/// * `unit` - A suffix indicating the unit of measurement.
pub fn ui_input_box(ui: &mut egui::Ui, label: &str, value: &mut f32, unit: &str) {
    ui_input_box_range(ui, label, value, unit, f32::MIN..=f32::MAX);
}

/// Renders a styled container holding a label and a `u32` `DragValue` input with an inclusive range.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the value.
/// * `value` - A mutable reference to the `u32` value to edit.
/// * `unit` - A suffix indicating the unit of measurement.
/// * `range` - The inclusive range of valid values.
pub fn ui_input_box_u32_range(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    unit: &str,
    range: std::ops::RangeInclusive<u32>,
) {
    ui_labeled_box(ui, label, 140.0, |ui| {
        let label_clr = label_color(ui.visuals());
        if !unit.is_empty() {
            ui.label(
                egui::RichText::new(unit)
                    .size(10.0)
                    .color(label_clr.gamma_multiply(0.5)),
            );
            ui.add_space(2.0);
        }

        let response = ui.add(egui::DragValue::new(value).speed(1.0).range(range));
        if response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }
    });
}

/// Renders a styled container holding a label and a `u32` `DragValue` input.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the value.
/// * `value` - A mutable reference to the `u32` value to edit.
/// * `unit` - A suffix indicating the unit of measurement.
pub fn ui_input_box_u32(ui: &mut egui::Ui, label: &str, value: &mut u32, unit: &str) {
    ui_input_box_u32_range(ui, label, value, unit, 0..=u32::MAX);
}

/// Renders a styled container holding a label and a `u16` `DragValue` input with an inclusive range.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the value.
/// * `value` - A mutable reference to the `u16` value to edit.
/// * `unit` - A suffix indicating the unit of measurement.
/// * `range` - The inclusive range of valid values.
pub fn ui_input_box_u16_range(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u16,
    unit: &str,
    range: std::ops::RangeInclusive<u16>,
) {
    ui_labeled_box(ui, label, 140.0, |ui| {
        let label_clr = label_color(ui.visuals());
        if !unit.is_empty() {
            ui.label(
                egui::RichText::new(unit)
                    .size(10.0)
                    .color(label_clr.gamma_multiply(0.5)),
            );
            ui.add_space(2.0);
        }

        let response = ui.add(egui::DragValue::new(value).speed(1.0).range(range));
        if response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }
    });
}

/// Renders a styled container holding a label and a single-line string input.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the value.
/// * `value` - A mutable reference to the string to edit.
/// * `width` - The total width of the input container.
pub fn ui_input_box_string(ui: &mut egui::Ui, label: &str, value: &mut String, width: f32) {
    ui_labeled_box(ui, label, width, |ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .margin(egui::vec2(4.0, 2.0))
                .frame(false)
                .horizontal_align(egui::Align::RIGHT),
        );
    });
}

/// Renders a styled container holding a label and a `u16` `DragValue` input.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the value.
/// * `value` - A mutable reference to the `u16` value to edit.
/// * `unit` - A suffix indicating the unit of measurement.
pub fn ui_input_box_u16(ui: &mut egui::Ui, label: &str, value: &mut u16, unit: &str) {
    ui_input_box_u16_range(ui, label, value, unit, 0..=u16::MAX);
}

/// Renders a wide, right-aligned setting row with a label and a drag input.
///
/// Features a left-aligned label and a right-aligned input box to keep long parameter
/// lists visually neat. Typically used in the Filters tab.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `label` - The description label of the setting.
/// * `value` - A mutable reference to the `f32` value to edit.
/// * `unit` - A suffix indicating the unit of measurement.
pub fn ui_setting_row(ui: &mut egui::Ui, label: &str, value: &mut f32, unit: &str) {
    ui_setting_row_range(ui, label, value, unit, f32::MIN..=f32::MAX);
}

/// Same as [`ui_setting_row`], but clamps the drag input to `range` instead of
/// allowing any value to be dragged/typed in.
pub fn ui_setting_row_range(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    unit: &str,
    range: std::ops::RangeInclusive<f32>,
) {
    let visuals = ui.visuals();
    let bg_fill = panel_bg(visuals);
    let border_color = panel_border(visuals);
    let label_clr = label_color(visuals);

    ui.scope(|ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(8.0, 3.0);

        egui::Frame::new()
            .fill(bg_fill)
            .corner_radius(4.0)
            .stroke(egui::Stroke::new(1.0_f32, border_color.gamma_multiply(0.6)))
            .inner_margin(egui::Margin::symmetric(14, 8))
            .show(ui, |ui| {
                ui.set_min_width(220.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(label)
                            .size(11.5)
                            .color(label_clr)
                            .strong(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !unit.is_empty() {
                            ui.label(
                                egui::RichText::new(unit)
                                    .size(10.5)
                                    .color(label_clr.gamma_multiply(0.5)),
                            );
                            ui.add_space(4.0);
                        }

                        let response = ui.add(
                            egui::DragValue::new(value)
                                .speed(0.1)
                                .range(range)
                                .max_decimals(2)
                                .custom_formatter(|val, _| {
                                    format!("{val:.2}")
                                        .trim_end_matches('0')
                                        .trim_end_matches('.')
                                        .replace('.', ",")
                                }),
                        );
                        if response.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                        }
                    });
                });
            });
    });
}

/// Renders a standard modern card with a title and icon for grouping multiple settings.
///
/// # Arguments
/// * `ui` - The egui user interface builder context.
/// * `title` - The title text of the card.
/// * `icon` - An icon character or string prefix.
/// * `add_contents` - A closure defining the contents to be rendered inside the card.
pub fn ui_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    icon: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    let visuals = ui.visuals();
    let card_bg = panel_bg(visuals).gamma_multiply(0.6);
    let border_color = panel_border(visuals).gamma_multiply(0.4);

    egui::Frame::new()
        .fill(card_bg)
        .corner_radius(4.0)
        .stroke(egui::Stroke::new(1.0_f32, border_color))
        .inner_margin(egui::Margin::symmetric(20, 15))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{icon} {title}"))
                            .size(14.0)
                            .strong(),
                    );
                });

                ui.add_space(10.0);
                add_contents(ui);
            });
        });
}
