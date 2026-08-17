//! Semantic color palette and global egui style application.

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
    /// Fill opacity multiplier used for the osu! playfield overlay.
    pub playfield_opacity: f32,
}

impl SemanticColors {
    #[must_use]
    pub const fn default(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                success: egui::Color32::from_rgb(166, 227, 161),
                warning: egui::Color32::from_rgb(249, 226, 175),
                error: egui::Color32::from_rgb(243, 139, 168),
                info: egui::Color32::from_rgb(137, 180, 250),
                playfield: egui::Color32::from_rgb(255, 105, 180),
                playfield_opacity: 0.25,
            }
        } else {
            Self {
                success: egui::Color32::from_rgb(64, 160, 43),
                warning: egui::Color32::from_rgb(223, 142, 29),
                error: egui::Color32::from_rgb(210, 15, 57),
                info: egui::Color32::from_rgb(30, 102, 245),
                playfield: egui::Color32::from_rgb(255, 105, 180),
                playfield_opacity: 0.25,
            }
        }
    }
}

/// Injects custom spacing, colors, and strokes into the `egui::Context`.
/// Called once at application startup.
pub fn apply_theme(ctx: &egui::Context, theme: &ThemePreference) {
    let mut semantic = SemanticColors::default(true);
    let mut style = (*ctx.style()).clone();

    // Default spacing & rounding (applied BEFORE theme, so Custom themes can override)
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
            if let Some(config) = crate::settings::themes::load_custom_theme(name) {
                semantic = SemanticColors::default(config.colors.dark_mode);

                let update_color = |opt: &Option<String>, target: &mut egui::Color32| {
                    if let Some(c) = opt {
                        *target =
                            crate::core::config::theme_models::ThemeConfig::parse_color(c, *target);
                    }
                };

                update_color(&config.colors.success_color, &mut semantic.success);
                update_color(&config.colors.warning_color, &mut semantic.warning);
                update_color(&config.colors.error_color, &mut semantic.error);
                update_color(&config.colors.info_color, &mut semantic.info);
                update_color(&config.colors.playfield_color, &mut semantic.playfield);

                if let Some(opacity) = config.colors.playfield_opacity {
                    semantic.playfield_opacity = opacity.clamp(0.0, 1.0);
                }

                // config.to_style takes the current style (which now has our default spacing)
                // and overrides it with any custom spacing in the theme!
                style = config.to_style(&style);
                style.visuals.clone()
            } else {
                egui::Visuals::dark()
            }
        }
    };

    let accent_color = visuals.selection.bg_fill;
    style.visuals = visuals.clone();

    // Standard NextTabletDriver widget interactive strokes
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
        0.5_f32,
        visuals
            .widgets
            .noninteractive
            .bg_stroke
            .color
            .gamma_multiply(0.5),
    );
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0_f32, accent_color.gamma_multiply(0.5));
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, accent_color);

    // Only apply the gamma_multiply background hover if the theme isn't explicitly overriding it.
    // Egui's default hovered bg is transparent for inactive widgets, but we want it tinted.
    style.visuals.widgets.hovered.bg_fill = visuals.widgets.hovered.bg_fill.gamma_multiply(0.8);

    style.visuals.selection.bg_fill = accent_color;
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, visuals.strong_text_color());

    ctx.set_style(style);

    // Store semantic colors in context memory
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("SemanticColors"), semantic));
}

/// Helper function to retrieve the active semantic colors
#[must_use]
pub fn semantic_colors(ctx: &egui::Context) -> SemanticColors {
    ctx.data(|d| {
        d.get_temp(egui::Id::new("SemanticColors"))
            .unwrap_or_else(|| SemanticColors::default(ctx.style().visuals.dark_mode))
    })
}

/// Returns a color for panel backgrounds that adapts to dark/light mode.
#[must_use]
pub const fn panel_bg(visuals: &egui::Visuals) -> egui::Color32 {
    visuals.panel_fill
}

/// Returns a color for panel borders that adapts to dark/light mode.
#[must_use]
pub const fn panel_border(visuals: &egui::Visuals) -> egui::Color32 {
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
