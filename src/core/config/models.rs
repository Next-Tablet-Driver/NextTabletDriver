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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
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

/// Configuration for the Devocub Antichatter implementation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AntichatterConfig {
    /// Whether the antichatter filter is enabled.
    pub enabled: bool,
    /// The latency buffer (in milliseconds) used for smoothing.
    pub latency: f32,
    /// The primary strength parameter of the Devocub antichatter algorithm.
    pub antichatter_strength: f32,
    /// Multiplier scaling the strength of the antichatter filter.
    pub antichatter_multiplier: f32,
    /// Horizontal offset parameter for the antichatter boundary.
    pub antichatter_offset_x: f32,
    /// Vertical offset parameter for the antichatter boundary.
    pub antichatter_offset_y: f32,
    /// Whether cursor prediction/extrapolation is enabled.
    pub prediction_enabled: bool,
    /// Strength of the cursor prediction algorithm.
    pub prediction_strength: f32,
    /// Sharpness parameter of the prediction curve.
    pub prediction_sharpness: f32,
    /// Horizontal offset for the prediction model.
    pub prediction_offset_x: f32,
    /// Vertical offset for the prediction model.
    pub prediction_offset_y: f32,
    /// Expected input frequency (in Hz) of the tablet packets.
    pub frequency: f32,
}

impl Default for AntichatterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            latency: 2.0,
            antichatter_strength: 3.0,
            antichatter_multiplier: 1.0,
            antichatter_offset_x: 0.0,
            antichatter_offset_y: 1.0,
            prediction_enabled: false,
            prediction_strength: 1.1,
            prediction_sharpness: 1.0,
            prediction_offset_x: 3.0,
            prediction_offset_y: 0.3,
            frequency: 1000.0,
        }
    }
}

/// Units used for reporting pen speed telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpeedUnit {
    /// Millimeters per second.
    #[default]
    MillimetersPerSecond,
    /// Meters per second.
    MetersPerSecond,
    /// Kilometers per hour.
    KilometersPerHour,
    /// Miles per hour.
    MilesPerHour,
}

/// Configuration for the Speed Statistics UDP telemetry sender.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeedStatsConfig {
    /// Whether the speed telemetry sender is enabled.
    pub enabled: bool,
    /// The destination IP address for the UDP statistics.
    pub ip: String,
    /// The destination UDP port for the telemetry packets.
    pub port: u16,
    /// The unit of measurement to format and send speed statistics.
    pub unit: SpeedUnit,
}

impl Default for SpeedStatsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ip: "127.0.0.1".to_string(),
            port: 9001,
            unit: SpeedUnit::MillimetersPerSecond,
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
    /// Antichatter filter settings for smoothing coordinates.
    #[serde(default)]
    pub antichatter: AntichatterConfig,
    /// Configuration for speed telemetry.
    #[serde(default)]
    pub speed_stats: SpeedStatsConfig,
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
}

impl Default for MappingConfig {
    fn default() -> Self {
        Self {
            mode: DriverMode::default(),
            active_area: ActiveArea::default(),
            target_area: TargetArea::default(),
            relative_config: RelativeConfig::default(),
            antichatter: AntichatterConfig::default(),
            speed_stats: SpeedStatsConfig::default(),
            tip_threshold: default_threshold(),
            eraser_threshold: default_threshold(),
            disable_pressure: false,
            disable_tilt: false,
            tip_binding: default_tip_binding(),
            eraser_binding: default_eraser_binding(),
            pen_button_bindings: default_button_bindings(),
            run_at_startup: false,
            system_tray_on_minimize: false,
            websocket: WebSocketConfig::default(),
            lock_aspect_ratio: false,
            show_osu_playfield: false,
            display_snapping: default_true(),
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

        // Active Area
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

        // Target Area
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

        // Antichatter
        if self.antichatter.frequency <= 0.0 || self.antichatter.frequency > 10000.0 {
            corrections.push(format!(
                "antichatter.frequency was invalid ({}), reset to {}",
                self.antichatter.frequency, defaults.antichatter.frequency
            ));
            self.antichatter.frequency = defaults.antichatter.frequency;
        }
        if self.antichatter.latency < 0.0 || self.antichatter.latency > 1000.0 {
            corrections.push(format!(
                "antichatter.latency was invalid ({}), reset to {}",
                self.antichatter.latency, defaults.antichatter.latency
            ));
            self.antichatter.latency = defaults.antichatter.latency;
        }
        if self.antichatter.antichatter_strength < 0.0 {
            self.antichatter.antichatter_strength = 0.0;
            corrections.push("antichatter_strength was negative, reset to 0".to_string());
        }

        // Relative Config
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

        // Thresholds
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

        // Network & IO
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
        if self.speed_stats.port == 0 {
            corrections.push(format!(
                "speed_stats.port was 0, reset to {}",
                defaults.speed_stats.port
            ));
            self.speed_stats.port = defaults.speed_stats.port;
        }

        // Buttons
        if self.pen_button_bindings.is_empty() {
            self.pen_button_bindings = defaults.pen_button_bindings;
            corrections.push("pen_button_bindings was empty, reset to defaults".to_string());
        } else if self.pen_button_bindings.len() > 32 {
            self.pen_button_bindings.truncate(32);
            corrections.push("pen_button_bindings was too long, truncated to 32".to_string());
        }

        corrections
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

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
}
