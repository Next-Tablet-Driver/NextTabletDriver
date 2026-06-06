//! # Visual Theme and Custom Widgets
//!
//! This module configures the egui global styling to create a clean, modern
//! Light theme that visually aligns with the aesthetics of `OpenTabletDriver` (OTD).
//! It also provides reusable helper functions for consistent layout paradigms
//! (like section headers and standardized input boxes) across panels.

use crate::core::config::models::ThemePreference;

use eframe::egui;

#[derive(Clone, Copy)]
pub struct SemanticColors {
    pub success: egui::Color32,
    pub warning: egui::Color32,
    pub error: egui::Color32,
    pub info: egui::Color32,
    /// Color used for the osu! playfield overlay in the tablet preview.
    /// Defaults to the iconic osu! pink so it is recognizable out of the box.
    pub playfield: egui::Color32,
}

impl SemanticColors {
    pub fn default(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                success: egui::Color32::from_rgb(166, 227, 161),
                warning: egui::Color32::from_rgb(249, 226, 175),
                error: egui::Color32::from_rgb(243, 139, 168),
                info: egui::Color32::from_rgb(137, 180, 250),
                playfield: egui::Color32::from_rgb(255, 105, 180),
            }
        } else {
            Self {
                success: egui::Color32::from_rgb(64, 160, 43),
                warning: egui::Color32::from_rgb(223, 142, 29),
                error: egui::Color32::from_rgb(210, 15, 57),
                info: egui::Color32::from_rgb(30, 102, 245),
                playfield: egui::Color32::from_rgb(255, 105, 180),
            }
        }
    }
}

/// Injects custom spacing, colors, and strokes into the `egui::Context`.
/// Called once at application startup.
pub fn apply_theme(ctx: &egui::Context, theme: &ThemePreference) {
    let mut semantic = SemanticColors::default(true);
    let visuals = match theme {
        ThemePreference::Light => {
            semantic = SemanticColors::default(false);
            let mut v = egui::Visuals::light();
            v.panel_fill = egui::Color32::from_gray(250);
            v.widgets.noninteractive.bg_stroke.color = egui::Color32::from_gray(235);
            v.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
            v
        }
        ThemePreference::Dark => {
            semantic = SemanticColors::default(true);
            let mut v = egui::Visuals::dark();
            v.panel_fill = egui::Color32::from_gray(45);
            v.widgets.noninteractive.bg_stroke.color = egui::Color32::from_gray(60);
            v.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
            v
        }
        ThemePreference::System => {
            let mut v = egui::Visuals::default();
            semantic = SemanticColors::default(v.dark_mode);
            if v.dark_mode {
                v.panel_fill = egui::Color32::from_gray(45);
                v.widgets.noninteractive.bg_stroke.color = egui::Color32::from_gray(60);
            } else {
                v.panel_fill = egui::Color32::from_gray(250);
                v.widgets.noninteractive.bg_stroke.color = egui::Color32::from_gray(235);
            }
            v.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
            v
        }
        ThemePreference::CatppuccinLatte => {
            semantic = SemanticColors::default(false);
            catppuccin_egui::set_theme(ctx, catppuccin_egui::LATTE);
            let mut v = ctx.style().visuals.clone();
            v.selection.bg_fill = catppuccin_egui::LATTE.blue;
            v
        }
        ThemePreference::CatppuccinFrappe => {
            semantic = SemanticColors::default(true);
            catppuccin_egui::set_theme(ctx, catppuccin_egui::FRAPPE);
            let mut v = ctx.style().visuals.clone();
            v.selection.bg_fill = catppuccin_egui::FRAPPE.blue;
            v
        }
        ThemePreference::CatppuccinMacchiato => {
            semantic = SemanticColors::default(true);
            catppuccin_egui::set_theme(ctx, catppuccin_egui::MACCHIATO);
            let mut v = ctx.style().visuals.clone();
            v.selection.bg_fill = catppuccin_egui::MACCHIATO.blue;
            v
        }
        ThemePreference::CatppuccinMocha => {
            semantic = SemanticColors::default(true);
            catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);
            let mut v = ctx.style().visuals.clone();
            v.selection.bg_fill = catppuccin_egui::MOCHA.blue;
            v
        }
        ThemePreference::Custom(name) => {
            let v = egui::Visuals::dark();
            if let Some(config) = crate::settings::themes::load_custom_theme(name) {
                semantic = SemanticColors::default(config.colors.dark_mode);
                
                if let Some(ref c) = config.colors.success_color {
                    semantic.success = crate::core::config::theme_models::ThemeConfig::parse_color(c, semantic.success);
                }
                if let Some(ref c) = config.colors.warning_color {
                    semantic.warning = crate::core::config::theme_models::ThemeConfig::parse_color(c, semantic.warning);
                }
                if let Some(ref c) = config.colors.error_color {
                    semantic.error = crate::core::config::theme_models::ThemeConfig::parse_color(c, semantic.error);
                }
                if let Some(ref c) = config.colors.info_color {
                    semantic.info = crate::core::config::theme_models::ThemeConfig::parse_color(c, semantic.info);
                }
                // Playfield color: use explicit override, otherwise keep the default osu! pink.
                if let Some(ref c) = config.colors.playfield_color {
                    semantic.playfield = crate::core::config::theme_models::ThemeConfig::parse_color(c, semantic.playfield);
                }
                
                let style = config.to_style(&ctx.style());
                ctx.set_style(style);
                
                // Return early so we don't overwrite the custom style with the hardcoded
                // spacing and corner radius below.
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("SemanticColors"), semantic));
                return;
            }
            v
        }
    };

    let accent_color = visuals.selection.bg_fill;

    let mut style = (*ctx.style()).clone();
    
    // Apply the newly constructed visuals to the style first!
    // This prevents default Egui dark theme widgets from overriding light/custom themes.
    style.visuals = visuals.clone();

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
        visuals
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
        visuals.widgets.hovered.bg_fill.gamma_multiply(0.8);

    style.visuals.selection.bg_fill = accent_color;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, visuals.strong_text_color());

    ctx.set_style(style);
    
    // Store semantic colors in context memory
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("SemanticColors"), semantic));
}

/// Helper function to retrieve the active semantic colors
pub fn semantic_colors(ctx: &egui::Context) -> SemanticColors {
    ctx.data(|d| d.get_temp(egui::Id::new("SemanticColors")).unwrap_or_else(|| SemanticColors::default(ctx.style().visuals.dark_mode)))
}

/// Returns a color for panel backgrounds that adapts to dark/light mode.
#[must_use]
pub fn panel_bg(visuals: &egui::Visuals) -> egui::Color32 {
    visuals.panel_fill
}

/// Returns a color for panel borders that adapts to dark/light mode.
#[must_use]
pub fn panel_border(visuals: &egui::Visuals) -> egui::Color32 {
    visuals.widgets.noninteractive.bg_stroke.color
}

/// Returns a subtle text color for labels.
#[must_use]
pub fn label_color(visuals: &egui::Visuals) -> egui::Color32 {
    visuals.text_color().gamma_multiply(0.7)
}

/// Returns the accent background color (blue area) that adapts to theme.
#[must_use]
pub fn accent_bg(visuals: &egui::Visuals) -> egui::Color32 {
    visuals.selection.bg_fill.gamma_multiply(0.8)
}

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
    let visuals = ui.visuals();
    let bg_fill = panel_bg(visuals);
    let border_color = panel_border(visuals);
    let label_clr = label_color(visuals);

    ui.scope(|ui| {
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
