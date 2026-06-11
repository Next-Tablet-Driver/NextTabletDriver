use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThemeMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub update_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThemeColors {
    pub dark_mode: bool,
    pub panel_bg: String,
    pub window_bg: String,
    pub text_color: String,
    pub strong_text_color: String,
    pub accent_color: String,
    pub border_color: String,
    pub widget_bg: String,
    pub widget_hover: String,
    pub widget_active: String,

    // Semantic colors for statuses
    pub success_color: Option<String>,
    pub warning_color: Option<String>,
    pub error_color: Option<String>,
    pub info_color: Option<String>,

    // Dedicated color for the osu! playfield overlay on the tablet preview.
    // If omitted, falls back to `info_color` (or the theme's default info color).
    pub playfield_color: Option<String>,
    // Opacity multiplier for the osu! playfield fill. Defaults to 0.25.
    pub playfield_opacity: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThemeSpacing {
    pub corner_radius: Option<f32>,
    pub item_spacing_x: Option<f32>,
    pub item_spacing_y: Option<f32>,
    pub button_padding_x: Option<f32>,
    pub button_padding_y: Option<f32>,
    pub border_width: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThemeConfig {
    pub metadata: ThemeMetadata,
    pub colors: ThemeColors,
    pub spacing: Option<ThemeSpacing>,
}

impl ThemeConfig {
    /// Converts a hex string "#RRGGBB" or "#RRGGBBAA" to `egui::Color32`
    #[must_use]
    pub fn parse_color(hex: &str, fallback: egui::Color32) -> egui::Color32 {
        let hex = hex.trim_start_matches('#');
        let len = hex.len();
        if len == 6 || len == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let a = if len == 8 {
                u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
            } else {
                255
            };
            egui::Color32::from_rgba_unmultiplied(r, g, b, a)
        } else {
            fallback
        }
    }

    /// Converts this custom theme configuration into an `egui::Style`
    #[must_use]
    pub fn to_style(&self, current_style: &egui::Style) -> egui::Style {
        let mut style = current_style.clone();

        let is_dark = self.colors.dark_mode;
        let mut visuals = if is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        // Parse colors with safe fallbacks from base visuals
        let panel_bg = Self::parse_color(&self.colors.panel_bg, visuals.panel_fill);
        let window_bg = Self::parse_color(&self.colors.window_bg, visuals.window_fill);
        let text_color = Self::parse_color(
            &self.colors.text_color,
            visuals.widgets.noninteractive.fg_stroke.color,
        );
        let strong_text_color = Self::parse_color(
            &self.colors.strong_text_color,
            visuals.widgets.active.fg_stroke.color,
        );
        let accent_color = Self::parse_color(&self.colors.accent_color, visuals.selection.bg_fill);
        let border_color = Self::parse_color(
            &self.colors.border_color,
            visuals.widgets.noninteractive.bg_stroke.color,
        );

        let widget_bg = Self::parse_color(&self.colors.widget_bg, visuals.widgets.inactive.bg_fill);
        let widget_hover =
            Self::parse_color(&self.colors.widget_hover, visuals.widgets.hovered.bg_fill);
        let widget_active =
            Self::parse_color(&self.colors.widget_active, visuals.widgets.active.bg_fill);

        visuals.panel_fill = panel_bg;
        visuals.window_fill = window_bg;
        visuals.extreme_bg_color = widget_bg; // Used for text inputs and drag values

        let border_w = self
            .spacing
            .as_ref()
            .and_then(|s| s.border_width)
            .unwrap_or(1.0);
        let noninteractive_border_w = self
            .spacing
            .as_ref()
            .and_then(|s| s.border_width)
            .map(|w| w * 0.5)
            .map_or(0.5, |w| w * 0.5);

        visuals.widgets.noninteractive.bg_fill = window_bg;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(border_w, text_color);
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(noninteractive_border_w, border_color);

        visuals.widgets.inactive.bg_fill = widget_bg;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(border_w, text_color);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

        visuals.widgets.hovered.bg_fill = widget_hover;
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(border_w, strong_text_color);
        visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(border_w, accent_color.gamma_multiply(0.5));

        visuals.widgets.active.bg_fill = widget_active;
        visuals.widgets.active.fg_stroke = egui::Stroke::new(border_w, strong_text_color);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(border_w, accent_color);

        visuals.selection.bg_fill = accent_color;
        visuals.selection.stroke = egui::Stroke::new(border_w, strong_text_color);

        // Apply Spacing & Rounding overrides if provided
        if let Some(spacing) = &self.spacing {
            if let Some(radius) = spacing.corner_radius {
                let corner_radius = egui::CornerRadius::same(radius as u8);
                visuals.widgets.noninteractive.corner_radius = corner_radius;
                visuals.widgets.inactive.corner_radius = corner_radius;
                visuals.widgets.hovered.corner_radius = corner_radius;
                visuals.widgets.active.corner_radius = corner_radius;
                visuals.widgets.open.corner_radius = corner_radius;
                visuals.window_corner_radius = corner_radius;
            }
            if let (Some(x), Some(y)) = (spacing.item_spacing_x, spacing.item_spacing_y) {
                style.spacing.item_spacing = egui::vec2(x, y);
            }
            if let (Some(x), Some(y)) = (spacing.button_padding_x, spacing.button_padding_y) {
                style.spacing.button_padding = egui::vec2(x, y);
            }
        }

        style.visuals = visuals;

        style
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::bool_assert_comparison
)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color() {
        let fallback = egui::Color32::from_rgb(255, 0, 0);

        // #RRGGBB format
        assert_eq!(
            ThemeConfig::parse_color("#00ff00", fallback),
            egui::Color32::from_rgb(0, 255, 0)
        );
        // Without # format
        assert_eq!(
            ThemeConfig::parse_color("0000ff", fallback),
            egui::Color32::from_rgb(0, 0, 255)
        );
        // #RRGGBBAA format
        assert_eq!(
            ThemeConfig::parse_color("#ff00ff80", fallback),
            egui::Color32::from_rgba_unmultiplied(255, 0, 255, 128)
        );
        // Invalid hex length (too short) -> fallback
        assert_eq!(ThemeConfig::parse_color("#fff", fallback), fallback);
        // Invalid characters (radix parses fail, resulting in 0)
        assert_eq!(
            ThemeConfig::parse_color("#zzzzzz", fallback),
            egui::Color32::from_rgb(0, 0, 0)
        );
    }

    #[test]
    fn test_theme_config_deserialization() {
        let json_content = r##"{
            "metadata": {
                "name": "CustomDark",
                "author": "Tester",
                "version": "1.0.0",
                "update_url": null
            },
            "colors": {
                "dark_mode": true,
                "panel_bg": "#1e1e1e",
                "window_bg": "#121212",
                "text_color": "#e0e0e0",
                "strong_text_color": "#ffffff",
                "accent_color": "#0078d7",
                "border_color": "#333333",
                "widget_bg": "#252525",
                "widget_hover": "#303030",
                "widget_active": "#353535",
                "success_color": "#00ff00",
                "warning_color": "#ffaa00",
                "error_color": "#ff0000",
                "info_color": "#0078d7",
                "playfield_color": "#ff69b4",
                "playfield_opacity": 0.3
            },
            "spacing": {
                "corner_radius": 6.0,
                "item_spacing_x": 8.0,
                "item_spacing_y": 8.0,
                "button_padding_x": 10.0,
                "button_padding_y": 5.0,
                "border_width": 1.5
            }
        }"##;

        let config: Result<ThemeConfig, _> = serde_json::from_str(json_content);
        assert!(config.is_ok());
        let theme = config.unwrap();
        assert_eq!(theme.metadata.name, "CustomDark");
        assert_eq!(theme.colors.dark_mode, true);
        assert_eq!(theme.colors.panel_bg, "#1e1e1e");
        assert_eq!(theme.spacing.as_ref().unwrap().corner_radius, Some(6.0));
    }

    #[test]
    fn test_to_style_mapping() {
        let theme = ThemeConfig {
            metadata: ThemeMetadata {
                name: "TestTheme".to_string(),
                author: "Tester".to_string(),
                version: "1.0.0".to_string(),
                update_url: None,
            },
            colors: ThemeColors {
                dark_mode: true,
                panel_bg: "#1e1e1e".to_string(),
                window_bg: "#121212".to_string(),
                text_color: "#e0e0e0".to_string(),
                strong_text_color: "#ffffff".to_string(),
                accent_color: "#0078d7".to_string(),
                border_color: "#333333".to_string(),
                widget_bg: "#252525".to_string(),
                widget_hover: "#303030".to_string(),
                widget_active: "#353535".to_string(),
                success_color: None,
                warning_color: None,
                error_color: None,
                info_color: None,
                playfield_color: None,
                playfield_opacity: None,
            },
            spacing: Some(ThemeSpacing {
                corner_radius: Some(8.0),
                item_spacing_x: Some(12.0),
                item_spacing_y: Some(12.0),
                button_padding_x: Some(14.0),
                button_padding_y: Some(6.0),
                border_width: Some(2.0),
            }),
        };

        let base_style = egui::Style::default();
        let style = theme.to_style(&base_style);

        // Verify that spacing attributes are applied correctly
        assert_eq!(style.spacing.item_spacing.x, 12.0);
        assert_eq!(style.spacing.item_spacing.y, 12.0);
        assert_eq!(style.spacing.button_padding.x, 14.0);
        assert_eq!(style.spacing.button_padding.y, 6.0);

        // Verify corner radius (rounding) on widgets
        let expected_radius = egui::CornerRadius::same(8);
        assert_eq!(
            style.visuals.widgets.inactive.corner_radius,
            expected_radius
        );
    }
}
