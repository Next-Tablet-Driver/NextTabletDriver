//! # Configuration Models
//!
//! This module defines the data structures used to serialize and deserialize
//! the application's configuration state (typically saved to `settings.json`).
//! It includes models for tablet mapping areas, UI preferences, and filter settings.

use serde::{Deserialize, Serialize};

/// Represents the absolute physical mapping area on the tablet surface.
///
/// All spatial coordinates in this struct are in **millimeters**.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveArea {
    /// Horizontal offset from the center of the tablet surface.
    pub x: f32,
    /// Vertical offset from the center of the tablet surface.
    pub y: f32,
    /// Total width of the mapping zone.
    pub w: f32,
    /// Total height of the mapping zone.
    pub h: f32,
    /// Clockwise rotation of the active area in degrees.
    pub rotation: f32,
}

impl ActiveArea {
    /// Normalizes rotation to [0, 360).
    pub fn normalize_rotation(&mut self) {
        self.rotation %= 360.0;
        if self.rotation < 0.0 {
            self.rotation += 360.0;
        }
    }

    /// Clamps the area dimensions and position to fit within the physical tablet surface.
    pub fn clamp_to_surface(&mut self, phys_w: f32, phys_h: f32) {
        self.w = self.w.clamp(1.0, phys_w);
        self.h = self.h.clamp(1.0, phys_h);
        self.x = self.x.clamp(self.w / 2.0, phys_w - self.w / 2.0);
        self.y = self.y.clamp(self.h / 2.0, phys_h - self.h / 2.0);

        self.normalize_rotation();
    }

    /// Adjusts the width or height to match the target aspect ratio, ensuring it stays within physical limits.
    pub fn apply_aspect_ratio(
        &mut self,
        target_ratio: f32,
        prefer_width: bool,
        phys_w: f32,
        phys_h: f32,
    ) {
        if prefer_width {
            self.h = (self.w / target_ratio).clamp(1.0, phys_h);
        } else {
            self.w = (self.h * target_ratio).clamp(1.0, phys_w);
        }
    }
}

impl Default for ActiveArea {
    fn default() -> Self {
        Self {
            x: 80.0,
            y: 50.0,
            w: 160.0,
            h: 100.0,
            rotation: 0.0,
        }
    }
}

/// Represents the target mapping area on the user's monitors.
///
/// All coordinates in this struct are in absolute virtual **pixels**
/// spanning across all connected displays.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TargetArea {
    /// Horizontal pixel offset from the top-left of the virtual desktop.
    pub x: f32,
    /// Vertical pixel offset from the top-left of the virtual desktop.
    pub y: f32,
    /// Total width of the mapped screen region.
    pub w: f32,
    /// Total height of the mapped screen region.
    pub h: f32,
}

impl Default for TargetArea {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        }
    }
}

/// Determines how pen movement translates to cursor movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DriverMode {
    /// Maps specific points on the tablet to specific points on the screen.
    /// Primarily used for drawing and osu!.
    #[default]
    Absolute,
    /// Moves the cursor relative to its current position, similar to a mouse.
    Relative,
}

/// User preference for application theme.
///
/// Serialized as a plain string so the web frontend can bind it directly: the variant name
/// (`"Dark"`, `"CatppuccinMocha"`...) or `"custom:<id>"` for a user theme file. Reading is
/// lenient so existing profiles keep loading: the legacy object form `{"Custom": "name"}` is
/// accepted, and an unrecognised string falls back to `System` rather than rejecting the whole
/// profile.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
    Custom(String),
}

/// Prefix of a user theme in the serialized form.
const CUSTOM_THEME_PREFIX: &str = "custom:";

impl ThemePreference {
    /// The string stored in profiles and exchanged with the frontend.
    #[must_use]
    pub fn as_config_string(&self) -> String {
        match self {
            Self::System => "System".to_string(),
            Self::Light => "Light".to_string(),
            Self::Dark => "Dark".to_string(),
            Self::CatppuccinLatte => "CatppuccinLatte".to_string(),
            Self::CatppuccinFrappe => "CatppuccinFrappe".to_string(),
            Self::CatppuccinMacchiato => "CatppuccinMacchiato".to_string(),
            Self::CatppuccinMocha => "CatppuccinMocha".to_string(),
            Self::Custom(id) => format!("{CUSTOM_THEME_PREFIX}{id}"),
        }
    }

    /// Parses the stored string; anything unrecognised becomes [`ThemePreference::System`].
    #[must_use]
    pub fn from_config_string(value: &str) -> Self {
        if let Some(id) = value.strip_prefix(CUSTOM_THEME_PREFIX) {
            return Self::Custom(id.to_string());
        }
        match value {
            "Light" => Self::Light,
            "Dark" => Self::Dark,
            "CatppuccinLatte" => Self::CatppuccinLatte,
            "CatppuccinFrappe" => Self::CatppuccinFrappe,
            "CatppuccinMacchiato" => Self::CatppuccinMacchiato,
            "CatppuccinMocha" => Self::CatppuccinMocha,
            _ => Self::System,
        }
    }
}

impl Serialize for ThemePreference {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_config_string())
    }
}

impl<'de> Deserialize<'de> for ThemePreference {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Text(String),
            /// Form written by earlier versions: `{"Custom": "name"}`.
            Legacy {
                #[serde(rename = "Custom")]
                custom: String,
            },
        }

        Ok(match Repr::deserialize(deserializer)? {
            Repr::Text(text) => Self::from_config_string(&text),
            Repr::Legacy { custom } => Self::Custom(custom),
        })
    }
}

/// Settings specific to [`DriverMode::Relative`] operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelativeConfig {
    /// Pixels per millimeter on the horizontal axis.
    pub x_sensitivity: f32,
    /// Pixels per millimeter on the vertical axis.
    pub y_sensitivity: f32,
    /// Rotation applied to the movement vector in degrees.
    pub rotation: f32,
    /// Time in milliseconds before relative movement resets (prevents drift).
    pub reset_time_ms: u32,
}

impl RelativeConfig {
    /// Normalizes rotation to [0, 360).
    pub fn normalize_rotation(&mut self) {
        self.rotation %= 360.0;
        if self.rotation < 0.0 {
            self.rotation += 360.0;
        }
    }
}

impl Default for RelativeConfig {
    fn default() -> Self {
        Self {
            x_sensitivity: 10.0,
            y_sensitivity: 10.0,
            rotation: 0.0,
            reset_time_ms: 100,
        }
    }
}

const fn default_threshold() -> u16 {
    10
}
const fn default_false() -> bool {
    false
}
const fn default_true() -> bool {
    true
}
fn default_tip_binding() -> String {
    "Mouse Button Binding: (Button: Left)".to_string()
}
fn default_eraser_binding() -> String {
    "None".to_string()
}
fn default_button_bindings() -> Vec<String> {
    vec!["None".to_string(), "None".to_string()]
}
fn default_language() -> String {
    "English".to_string()
}
const fn default_ws_port() -> u16 {
    8080
}
const fn default_ws_hz() -> u32 {
    60
}

/// Configuration for the embedded WebSocket server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct WebSocketConfig {
    /// Whether the WebSocket server is enabled.
    #[serde(default = "default_false")]
    pub enabled: bool,
    /// The TCP port to bind the WebSocket server to.
    #[serde(default = "default_ws_port")]
    pub port: u16,
    /// The rate (in Hz) at which coordinate/status packets are sent to clients.
    #[serde(default = "default_ws_hz")]
    pub polling_rate_hz: u32,
    /// Whether to transmit x and y coordinates to clients.
    #[serde(default = "default_true")]
    pub send_coordinates: bool,
    /// Whether to transmit pen pressure values to clients.
    #[serde(default = "default_true")]
    pub send_pressure: bool,
    /// Whether to transmit tilt information to clients.
    #[serde(default = "default_true")]
    pub send_tilt: bool,
    /// Whether to transmit general status information (e.g. connections, proximity).
    #[serde(default = "default_true")]
    pub send_status: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: default_false(),
            port: default_ws_port(),
            polling_rate_hz: default_ws_hz(),
            send_coordinates: default_true(),
            send_pressure: default_true(),
            send_tilt: default_true(),
            send_status: default_true(),
        }
    }
}

/// Configuration settings for an individual dynamic plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DynamicPluginSettings {
    /// Whether the plugin is enabled in the filtering pipeline.
    #[serde(default = "default_false")]
    pub enabled: bool,
    /// Key-value bag of configuration property values serialized as JSON values.
    #[serde(default)]
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Determines how normalized pressure input `[0, 1]` is reshaped before being
/// reported to the OS/game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PressureCurveType {
    /// Reported pressure equals input pressure (identity curve).
    #[default]
    Linear,
    /// Reported pressure follows `input.powf(exponent)`.
    Exponential,
    /// Reported pressure follows a piecewise-linear interpolation through
    /// user-defined control points.
    Custom,
}

const fn default_curve_exponent() -> f32 {
    2.0
}
fn default_curve_points() -> Vec<(f32, f32)> {
    vec![(0.0, 0.0), (1.0, 1.0)]
}

/// Configuration for the pressure response curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureCurveConfig {
    /// Which curve shape to apply.
    #[serde(default)]
    pub curve_type: PressureCurveType,
    /// Exponent used when `curve_type` is [`PressureCurveType::Exponential`].
    #[serde(default = "default_curve_exponent")]
    pub exponent: f32,
    /// Control points used when `curve_type` is [`PressureCurveType::Custom`],
    /// sorted by `.0` (x), spanning the full `[0, 1]` domain.
    #[serde(default = "default_curve_points")]
    pub points: Vec<(f32, f32)>,
}

impl Default for PressureCurveConfig {
    fn default() -> Self {
        Self {
            curve_type: PressureCurveType::default(),
            exponent: default_curve_exponent(),
            points: default_curve_points(),
        }
    }
}

/// The root configuration struct for the application.
///
/// This structure holds all user-adjustable parameters and is the
/// primary object serialized to disk. Default struct fields are provided by individual functions
/// to facilitate serde compatibility for adding new fields to older config files.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct MappingConfig {
    /// The active tracking mode (Absolute or Relative).
    #[serde(default)]
    pub mode: DriverMode,
    /// The physical area on the tablet surface mapped to input.
    pub active_area: ActiveArea,
    /// The screen coordinates area that receives the mapped inputs.
    pub target_area: TargetArea,
    /// Custom settings for relative driver mode (sensitivity, rotation, reset time).
    #[serde(default)]
    pub relative_config: RelativeConfig,
    /// Pressure threshold above which the pen tip is considered active.
    #[serde(default = "default_threshold")]
    pub tip_threshold: u16,
    /// Pressure threshold above which the pen eraser is considered active.
    #[serde(default = "default_threshold")]
    pub eraser_threshold: u16,
    /// Whether to ignore all pen pressure data.
    #[serde(default = "default_false")]
    pub disable_pressure: bool,
    /// Whether to ignore all pen tilt data.
    #[serde(default = "default_false")]
    pub disable_tilt: bool,
    /// Binding string definition triggered when the pen tip contacts the tablet.
    #[serde(default = "default_tip_binding")]
    pub tip_binding: String,
    /// Binding string definition triggered when the pen eraser is used.
    #[serde(default = "default_eraser_binding")]
    pub eraser_binding: String,
    /// Dynamic binding definitions mapping pen buttons to key/mouse events.
    #[serde(default = "default_button_bindings")]
    pub pen_button_bindings: Vec<String>,
    /// Whether the application starts automatically when the system boots.
    #[serde(default = "default_false")]
    pub run_at_startup: bool,
    /// Whether minimizing the application window hides it to the system tray.
    #[serde(default = "default_false")]
    pub system_tray_on_minimize: bool,
    /// Whether to force the Windows system timer resolution to 0.5ms for low-latency tablet polling.
    #[serde(default = "default_true")]
    pub force_high_resolution_timer: bool,
    /// WebSocket server broadcast configuration.
    #[serde(default)]
    pub websocket: WebSocketConfig,
    /// Lock aspect ratio of the active area to match the target area screen aspect ratio.
    #[serde(default)]
    pub lock_aspect_ratio: bool,
    /// Whether to display a visual guide of the osu! playfield within the active area grid.
    #[serde(default)]
    pub show_osu_playfield: bool,
    /// Whether to snap the target area to display edges when resizing/moving.
    #[serde(default = "default_true")]
    pub display_snapping: bool,
    /// Pressure response curve settings.
    #[serde(default)]
    pub pressure_curve: PressureCurveConfig,
    /// User interface theme preference.
    #[serde(default)]
    pub theme: ThemePreference,
    /// User interface language preference.
    #[serde(default = "default_language")]
    pub language: String,
    /// Dynamic plugins configuration (keyed by plugin ID).
    #[serde(default)]
    pub plugins: std::collections::HashMap<String, DynamicPluginSettings>,
}

impl Default for MappingConfig {
    fn default() -> Self {
        Self {
            mode: DriverMode::default(),
            active_area: ActiveArea::default(),
            target_area: TargetArea::default(),
            relative_config: RelativeConfig::default(),
            tip_threshold: default_threshold(),
            eraser_threshold: default_threshold(),
            disable_pressure: false,
            disable_tilt: false,
            tip_binding: default_tip_binding(),
            eraser_binding: default_eraser_binding(),
            pen_button_bindings: default_button_bindings(),
            run_at_startup: false,
            system_tray_on_minimize: false,
            force_high_resolution_timer: true,
            websocket: WebSocketConfig::default(),
            lock_aspect_ratio: false,
            show_osu_playfield: false,
            display_snapping: default_true(),
            pressure_curve: PressureCurveConfig::default(),
            theme: ThemePreference::default(),
            language: default_language(),
            plugins: std::collections::HashMap::new(),
        }
    }
}

impl MappingConfig {
    /// Validates deserialized config values and repairs any invalid fields.
    ///
    /// Returns a list of human-readable correction messages. An empty list
    /// means the config was valid as-is.
    pub fn validate_and_repair(&mut self) -> Vec<String> {
        let mut corrections = Vec::new();
        let defaults = Self::default();

        if self.active_area.w <= 0.0 {
            corrections.push(format!(
                "active_area.w was invalid ({}), reset to {}",
                self.active_area.w, defaults.active_area.w
            ));
            self.active_area.w = defaults.active_area.w;
        }
        if self.active_area.h <= 0.0 {
            corrections.push(format!(
                "active_area.h was invalid ({}), reset to {}",
                self.active_area.h, defaults.active_area.h
            ));
            self.active_area.h = defaults.active_area.h;
        }
        let old_rotation = self.active_area.rotation;
        self.active_area.normalize_rotation();
        if (self.active_area.rotation - old_rotation).abs() > f32::EPSILON {
            corrections.push(format!(
                "active_area.rotation normalized from {} to {}",
                old_rotation, self.active_area.rotation
            ));
        }

        if self.target_area.w <= 0.0 || self.target_area.w > 100_000.0 {
            corrections.push(format!(
                "target_area.w was invalid ({}), reset to {}",
                self.target_area.w, defaults.target_area.w
            ));
            self.target_area.w = defaults.target_area.w;
        }
        if self.target_area.h <= 0.0 {
            corrections.push(format!(
                "target_area.h was invalid ({}), reset to {}",
                self.target_area.h, defaults.target_area.h
            ));
            self.target_area.h = defaults.target_area.h;
        }

        if self.relative_config.x_sensitivity <= 0.0 || self.relative_config.x_sensitivity > 1000.0
        {
            corrections.push(format!(
                "relative_config.x_sensitivity was invalid ({}), reset to {}",
                self.relative_config.x_sensitivity, defaults.relative_config.x_sensitivity
            ));
            self.relative_config.x_sensitivity = defaults.relative_config.x_sensitivity;
        }
        if self.relative_config.y_sensitivity <= 0.0 {
            corrections.push(format!(
                "relative_config.y_sensitivity was invalid ({}), reset to {}",
                self.relative_config.y_sensitivity, defaults.relative_config.y_sensitivity
            ));
            self.relative_config.y_sensitivity = defaults.relative_config.y_sensitivity;
        }
        let old_rel_rotation = self.relative_config.rotation;
        self.relative_config.normalize_rotation();
        if (self.relative_config.rotation - old_rel_rotation).abs() > f32::EPSILON {
            corrections.push(format!(
                "relative_config.rotation normalized from {} to {}",
                old_rel_rotation, self.relative_config.rotation
            ));
        }
        if self.relative_config.reset_time_ms == 0 || self.relative_config.reset_time_ms > 10000 {
            corrections.push(format!(
                "relative_config.reset_time_ms was invalid ({}), reset to {}",
                self.relative_config.reset_time_ms, defaults.relative_config.reset_time_ms
            ));
            self.relative_config.reset_time_ms = defaults.relative_config.reset_time_ms;
        }

        if self.tip_threshold == 0 || self.tip_threshold > 1024 {
            corrections.push(format!(
                "tip_threshold was {}, reset to {}",
                self.tip_threshold, defaults.tip_threshold
            ));
            self.tip_threshold = defaults.tip_threshold;
        }
        if self.eraser_threshold == 0 || self.eraser_threshold > 1024 {
            corrections.push(format!(
                "eraser_threshold was {}, reset to {}",
                self.eraser_threshold, defaults.eraser_threshold
            ));
            self.eraser_threshold = defaults.eraser_threshold;
        }

        if self.websocket.port == 0 {
            corrections.push(format!(
                "websocket.port was 0, reset to {}",
                defaults.websocket.port
            ));
            self.websocket.port = defaults.websocket.port;
        }
        if self.websocket.polling_rate_hz == 0 || self.websocket.polling_rate_hz > 1000 {
            corrections.push(format!(
                "websocket.polling_rate_hz was invalid ({}), reset to {}",
                self.websocket.polling_rate_hz, defaults.websocket.polling_rate_hz
            ));
            self.websocket.polling_rate_hz = defaults.websocket.polling_rate_hz;
        }

        if self.pen_button_bindings.is_empty() {
            self.pen_button_bindings = defaults.pen_button_bindings;
            corrections.push("pen_button_bindings was empty, reset to defaults".to_string());
        } else if self.pen_button_bindings.len() > 32 {
            self.pen_button_bindings.truncate(32);
            corrections.push("pen_button_bindings was too long, truncated to 32".to_string());
        }

        if !self.pressure_curve.exponent.is_finite()
            || !(0.1..=5.0).contains(&self.pressure_curve.exponent)
        {
            corrections.push(format!(
                "pressure_curve.exponent was invalid ({}), reset to {}",
                self.pressure_curve.exponent, defaults.pressure_curve.exponent
            ));
            self.pressure_curve.exponent = if self.pressure_curve.exponent.is_finite() {
                self.pressure_curve.exponent.clamp(0.1, 5.0)
            } else {
                defaults.pressure_curve.exponent
            };
        }
        if self.pressure_curve.points.len() < 2 {
            corrections.push(
                "pressure_curve.points had fewer than 2 entries, reset to defaults".to_string(),
            );
            self.pressure_curve.points = defaults.pressure_curve.points;
        } else {
            if self.pressure_curve.points.len() > 32 {
                self.pressure_curve.points.truncate(32);
                corrections.push("pressure_curve.points was too long, truncated to 32".to_string());
            }
            for p in &mut self.pressure_curve.points {
                p.0 = p.0.clamp(0.0, 1.0);
                p.1 = p.1.clamp(0.0, 1.0);
            }
            self.pressure_curve
                .points
                .sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some(first) = self.pressure_curve.points.first_mut() {
                first.0 = 0.0;
            }
            if let Some(last) = self.pressure_curve.points.last_mut() {
                last.0 = 1.0;
            }
        }

        corrections
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn theme_preference_round_trips_as_a_plain_string() {
        let all = [
            ThemePreference::System,
            ThemePreference::Light,
            ThemePreference::Dark,
            ThemePreference::CatppuccinLatte,
            ThemePreference::CatppuccinFrappe,
            ThemePreference::CatppuccinMacchiato,
            ThemePreference::CatppuccinMocha,
            ThemePreference::Custom("neon-night".to_string()),
        ];
        for theme in all {
            let json = serde_json::to_string(&theme).unwrap();
            assert!(json.starts_with('"'), "{json} must be a JSON string");
            assert_eq!(
                serde_json::from_str::<ThemePreference>(&json).unwrap(),
                theme
            );
        }
        assert_eq!(
            serde_json::to_string(&ThemePreference::Custom("neon".into())).unwrap(),
            "\"custom:neon\""
        );
        assert_eq!(
            serde_json::to_string(&ThemePreference::CatppuccinMocha).unwrap(),
            "\"CatppuccinMocha\""
        );
    }

    #[test]
    fn theme_preference_reads_legacy_and_unknown_values() {
        // Existing profiles stored the bare variant name: still valid.
        assert_eq!(
            serde_json::from_str::<ThemePreference>("\"Dark\"").unwrap(),
            ThemePreference::Dark
        );
        // The object form an earlier version would have written for a custom theme.
        assert_eq!(
            serde_json::from_str::<ThemePreference>("{\"Custom\":\"old\"}").unwrap(),
            ThemePreference::Custom("old".into())
        );
        // A value from a newer version must not make the whole profile unreadable.
        assert_eq!(
            serde_json::from_str::<ThemePreference>("\"Solarized\"").unwrap(),
            ThemePreference::System
        );
    }

    #[test]
    fn config_with_a_custom_theme_survives_a_round_trip() {
        let config = MappingConfig {
            theme: ThemePreference::Custom("neon".into()),
            ..MappingConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"theme\":\"custom:neon\""));
        let back: MappingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.theme, ThemePreference::Custom("neon".into()));
    }

    #[test]
    fn test_config_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let config = MappingConfig::default();
        let json = serde_json::to_string(&config)?;
        let deserialized: MappingConfig = serde_json::from_str(&json)?;
        assert_eq!(config, deserialized);
        Ok(())
    }

    #[test]
    fn test_normalize_rotation_active_area() {
        let mut area = ActiveArea {
            rotation: 450.0,
            ..Default::default()
        };
        area.normalize_rotation();
        assert!((area.rotation - 90.0).abs() < f32::EPSILON);

        area.rotation = -90.0;
        area.normalize_rotation();
        assert!((area.rotation - 270.0).abs() < f32::EPSILON);

        area.rotation = 360.0;
        area.normalize_rotation();
        assert!((area.rotation - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_rotation_relative_config() {
        let mut cfg = RelativeConfig {
            rotation: -45.0,
            ..Default::default()
        };
        cfg.normalize_rotation();
        assert!((cfg.rotation - 315.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_clamp_to_surface_calls_normalize() {
        let mut area = ActiveArea {
            x: 80.0,
            y: 50.0,
            w: 160.0,
            h: 100.0,
            rotation: 450.0,
        };
        area.clamp_to_surface(160.0, 100.0);
        assert!((area.rotation - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_apply_aspect_ratio() {
        let mut area = ActiveArea::default();
        // prefer_width: h should be derived from w
        area.apply_aspect_ratio(16.0 / 9.0, true, 200.0, 200.0);
        let expected_h = area.w / (16.0 / 9.0);
        assert!((area.h - expected_h).abs() < f32::EPSILON);
    }

    #[test]
    fn test_validate_and_repair() {
        let mut config = MappingConfig::default();

        config.active_area.w = -10.0;
        config.active_area.h = 0.0;
        config.active_area.rotation = 450.0;
        config.target_area.w = -5.0;
        config.relative_config.rotation = -90.0;
        config.relative_config.reset_time_ms = 20000;
        config.tip_threshold = 0;
        config.eraser_threshold = 2000;
        config.websocket.port = 0;
        config.websocket.polling_rate_hz = 5000;
        config.pen_button_bindings = vec![];

        let corrections = config.validate_and_repair();

        assert!(!corrections.is_empty());
        assert!(config.active_area.w > 0.0);
        assert!(config.active_area.h > 0.0);
        assert_eq!(config.active_area.rotation, 90.0);
        assert!(config.target_area.w > 0.0);
        assert_eq!(config.relative_config.rotation, 270.0);
        assert!(config.relative_config.reset_time_ms <= 10000);
        assert!(config.tip_threshold > 0);
        assert!(config.eraser_threshold <= 1024);
        assert_eq!(config.websocket.port, 8080);
        assert_eq!(config.websocket.polling_rate_hz, 60);
        assert!(!config.pen_button_bindings.is_empty());
    }
    #[test]
    fn test_validate_and_repair_pressure_curve() {
        let mut config = MappingConfig::default();

        config.pressure_curve.exponent = 100.0;
        config.pressure_curve.points = vec![(0.3, 0.9), (0.1, 0.2), (0.7, 0.1)];

        let corrections = config.validate_and_repair();

        assert!(!corrections.is_empty());
        assert!((0.1..=5.0).contains(&config.pressure_curve.exponent));
        assert_eq!(config.pressure_curve.points.first().unwrap().0, 0.0);
        assert_eq!(config.pressure_curve.points.last().unwrap().0, 1.0);
        assert!(
            config
                .pressure_curve
                .points
                .windows(2)
                .all(|w| matches!(w, [a, b] if a.0 <= b.0))
        );
    }
    #[test]
    fn test_validate_and_repair_pressure_curve_too_few_points() {
        let mut config = MappingConfig::default();
        config.pressure_curve.points = vec![(0.5, 0.5)];

        let corrections = config.validate_and_repair();

        assert!(!corrections.is_empty());
        assert_eq!(config.pressure_curve.points, vec![(0.0, 0.0), (1.0, 1.0)]);
    }

    #[test]
    fn the_height_is_derived_from_the_width_or_the_other_way_round() {
        let mut area = ActiveArea {
            x: 50.0,
            y: 50.0,
            w: 40.0,
            h: 30.0,
            rotation: 0.0,
        };
        area.apply_aspect_ratio(2.0, true, 100.0, 100.0);
        assert_eq!((area.w, area.h), (40.0, 20.0));
        area.apply_aspect_ratio(2.0, false, 100.0, 100.0);
        assert_eq!((area.w, area.h), (40.0, 20.0));
        area.h = 30.0;
        area.apply_aspect_ratio(2.0, false, 100.0, 100.0);
        assert_eq!((area.w, area.h), (60.0, 30.0));
    }

    #[test]
    fn repair_resets_each_invalid_numeric_setting_to_its_default() {
        let defaults = MappingConfig::default();
        let mut config = MappingConfig::default();
        config.target_area.h = -1.0;
        config.relative_config.x_sensitivity = 5000.0;
        config.relative_config.y_sensitivity = 0.0;

        let corrections = config.validate_and_repair();

        assert_eq!(corrections.len(), 3, "{corrections:?}");
        assert_eq!(config.target_area.h, defaults.target_area.h);
        assert_eq!(
            config.relative_config.x_sensitivity,
            defaults.relative_config.x_sensitivity
        );
        assert_eq!(
            config.relative_config.y_sensitivity,
            defaults.relative_config.y_sensitivity
        );
    }

    #[test]
    fn repair_keeps_pen_button_bindings_within_bounds() {
        let defaults = MappingConfig::default();
        let mut config = MappingConfig {
            pen_button_bindings: vec!["Left Click".to_string(); 40],
            ..MappingConfig::default()
        };
        assert_eq!(config.validate_and_repair().len(), 1);
        assert_eq!(config.pen_button_bindings.len(), 32);

        config.pen_button_bindings.clear();
        assert_eq!(config.validate_and_repair().len(), 1);
        assert_eq!(config.pen_button_bindings, defaults.pen_button_bindings);
    }

    #[test]
    fn repair_brings_the_pressure_exponent_back_into_range() {
        let defaults = MappingConfig::default();
        let mut config = MappingConfig::default();

        config.pressure_curve.exponent = f32::NAN;
        assert_eq!(config.validate_and_repair().len(), 1);
        assert_eq!(
            config.pressure_curve.exponent,
            defaults.pressure_curve.exponent
        );

        config.pressure_curve.exponent = 10.0;
        config.validate_and_repair();
        assert_eq!(config.pressure_curve.exponent, 5.0);

        config.pressure_curve.exponent = 0.0;
        config.validate_and_repair();
        assert_eq!(config.pressure_curve.exponent, 0.1);
    }

    #[test]
    fn repair_normalizes_the_curve_control_points() {
        let defaults = MappingConfig::default();
        let mut config = MappingConfig::default();

        config.pressure_curve.points = vec![(0.0, 0.5); 40];
        assert_eq!(config.validate_and_repair().len(), 1);
        assert_eq!(config.pressure_curve.points.len(), 32);

        config.pressure_curve.points = vec![(0.5, 0.5)];
        assert_eq!(config.validate_and_repair().len(), 1);
        assert_eq!(config.pressure_curve.points, defaults.pressure_curve.points);

        // Unsorted and out-of-range points are clamped, sorted and pinned to the unit square.
        config.pressure_curve.points = vec![(0.7, 2.0), (0.2, -1.0), (0.9, 0.5)];
        assert!(config.validate_and_repair().is_empty());
        assert_eq!(
            config.pressure_curve.points,
            vec![(0.0, 0.0), (0.7, 1.0), (1.0, 0.5)]
        );
    }
}
