//! # Visual Theme and Custom Widgets
//!
//! This module configures the egui global styling to create a clean, modern
//! Light theme that visually aligns with the aesthetics of `OpenTabletDriver` (OTD).
//! It also provides reusable helper functions for consistent layout paradigms
//! (like section headers and standardized input boxes) across panels.

use crate::core::config::models::ThemePreference;

use eframe::egui;

/// Injects custom spacing, colors, and strokes into the `egui::Context`.
/// Called once at application startup.
pub fn apply_theme(ctx: &egui::Context, theme: ThemePreference) {
    match theme {
        ThemePreference::Light => ctx.set_visuals(egui::Visuals::light()),
        ThemePreference::Dark => ctx.set_visuals(egui::Visuals::dark()),
        ThemePreference::System => ctx.set_visuals(egui::Visuals::default()),
        ThemePreference::CatppuccinLatte => catppuccin_egui::set_theme(ctx, catppuccin_egui::LATTE),
        ThemePreference::CatppuccinFrappe => {
            catppuccin_egui::set_theme(ctx, catppuccin_egui::FRAPPE);
        }
        ThemePreference::CatppuccinMacchiato => {
            catppuccin_egui::set_theme(ctx, catppuccin_egui::MACCHIATO);
        }
        ThemePreference::CatppuccinMocha => catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA),
    }

    // Determine the accent color based on theme
    let accent_color = match theme {
        ThemePreference::CatppuccinLatte => catppuccin_egui::LATTE.blue,
        ThemePreference::CatppuccinFrappe => catppuccin_egui::FRAPPE.blue,
        ThemePreference::CatppuccinMacchiato => catppuccin_egui::MACCHIATO.blue,
        ThemePreference::CatppuccinMocha => catppuccin_egui::MOCHA.blue,
        _ => egui::Color32::from_rgb(0, 120, 215),
    };

    let mut style = (*ctx.style()).clone();

    // Spacing & Rounding
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.interact_size.y = 20.0;

    let corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.noninteractive.corner_radius = corner_radius;
    style.visuals.widgets.inactive.corner_radius = corner_radius;
    style.visuals.widgets.hovered.corner_radius = corner_radius;
    style.visuals.widgets.active.corner_radius = corner_radius;
    style.visuals.widgets.open.corner_radius = corner_radius;
    style.visuals.window_corner_radius = corner_radius;

    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
        0.5,
        style
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .color
            .gamma_multiply(0.5),
    );
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, accent_color.gamma_multiply(0.5));
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent_color);

    style.visuals.widgets.hovered.bg_fill =
        style.visuals.widgets.hovered.bg_fill.gamma_multiply(0.8);

    style.visuals.selection.bg_fill = accent_color;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

    ctx.set_style(style);
}

/// Returns a color for panel backgrounds that adapts to dark/light mode.
#[must_use]
pub fn panel_bg(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.window_fill == egui::Visuals::dark().window_fill {
        egui::Color32::from_gray(45)
    } else if visuals.window_fill == egui::Visuals::light().window_fill {
        egui::Color32::from_gray(250)
    } else {
        visuals.panel_fill // Use the theme's own panel color (e.g., Mantle for Catppuccin)
    }
}

/// Returns a color for panel borders that adapts to dark/light mode.
#[must_use]
pub fn panel_border(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.window_fill == egui::Visuals::dark().window_fill {
        egui::Color32::from_gray(60)
    } else if visuals.window_fill == egui::Visuals::light().window_fill {
        egui::Color32::from_gray(235)
    } else {
        visuals.widgets.noninteractive.bg_stroke.color
    }
}

/// Returns a subtle text color for labels.
#[must_use]
pub fn label_color(visuals: &egui::Visuals) -> egui::Color32 {
    visuals.text_color().gamma_multiply(0.7)
}

/// Returns the accent background color (blue area) that adapts to theme.
#[must_use]
pub fn accent_bg(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_rgba_unmultiplied(60, 120, 180, 255) // Darker blue
    } else {
        egui::Color32::from_rgba_unmultiplied(137, 196, 244, 255) // Original light blue
    }
}

/// Renders a standardized section header with a title and a horizontal separator line.
pub fn ui_section_header(ui: &mut egui::Ui, title: &str) {
    let text_color = ui.visuals().strong_text_color();
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(egui::RichText::new(title).size(16.0).color(text_color));
    });
    ui.add(egui::Separator::default().spacing(4.0).grow(2.0));
}

/// Core helper for creating a styled container with a label and a right-aligned widget.
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
        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::from_white_alpha(15);
        ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::from_white_alpha(20);
        ui.style_mut().spacing.button_padding = egui::vec2(6.0, 2.0);

        egui::Frame::new()
            .fill(bg_fill)
            .corner_radius(4.0)
            .stroke(egui::Stroke::new(1.0, border_color.gamma_multiply(0.6)))
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

/// Renders a styled container holding a label and an `f32` `DragValue` input with an optional range.
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

pub fn ui_input_box(ui: &mut egui::Ui, label: &str, value: &mut f32, unit: &str) {
    ui_input_box_range(ui, label, value, unit, f32::MIN..=f32::MAX);
}

/// Renders a styled container holding a label and a `u32` `DragValue` input with an optional range.
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

pub fn ui_input_box_u32(ui: &mut egui::Ui, label: &str, value: &mut u32, unit: &str) {
    ui_input_box_u32_range(ui, label, value, unit, 0..=u32::MAX);
}

/// Renders a styled container holding a label and a `u16` `DragValue` input with an optional range.
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

/// Renders a styled container holding a label and a string input.
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

pub fn ui_input_box_u16(ui: &mut egui::Ui, label: &str, value: &mut u16, unit: &str) {
    ui_input_box_u16_range(ui, label, value, unit, 0..=u16::MAX);
}

/// Renders a wide, right-aligned setting row typically used in the Filters tab.
///
/// Features a left-aligned label and a right-aligned input box to keep long parameter
/// lists visually neat.
pub fn ui_setting_row(ui: &mut egui::Ui, label: &str, value: &mut f32, unit: &str) {
    let visuals = ui.visuals();
    let bg_fill = panel_bg(visuals);
    let border_color = panel_border(visuals);
    let label_clr = label_color(visuals);

    ui.scope(|ui| {
        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_white_alpha(8);
        ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::from_white_alpha(15);
        ui.style_mut().spacing.button_padding = egui::vec2(8.0, 3.0);

        egui::Frame::new()
            .fill(bg_fill)
            .corner_radius(4.0)
            .stroke(egui::Stroke::new(1.0, border_color.gamma_multiply(0.6)))
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
                                .max_decimals(2)
                                .custom_formatter(|val, _| {
                                    format!("{val:.2}")
                                        .trim_end_matches('0')
                                        .trim_end_matches('.')
                                        .replace('.', ",")
                                })
                                .clamp_existing_to_range(false),
                        );
                        if response.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                        }
                    });
                });
            });
    });
}

/// Renders a standard modern card for grouping settings.
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
        .stroke(egui::Stroke::new(1.0, border_color))
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
