use crate::core::config::models::{
    ActiveArea, DriverMode, MappingConfig, RelativeConfig, TargetArea,
};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Default)]
#[serde(default)]
struct OtdSettings {
    #[serde(rename = "Profiles")]
    profiles: Vec<OtdProfile>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OtdProfile {
    #[serde(rename = "OutputMode")]
    output_mode: Option<OtdOutputMode>,
    #[serde(rename = "AbsoluteModeSettings")]
    absolute_settings: Option<OtdAbsoluteSettings>,
    #[serde(rename = "RelativeModeSettings")]
    relative_settings: Option<OtdRelativeSettings>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OtdOutputMode {
    #[serde(rename = "Path")]
    path: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OtdAbsoluteSettings {
    #[serde(rename = "Display")]
    display: OtdArea,
    #[serde(rename = "Tablet")]
    tablet: OtdArea,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OtdArea {
    #[serde(rename = "Width")]
    width: f32,
    #[serde(rename = "Height")]
    height: f32,
    #[serde(rename = "X")]
    x: f32,
    #[serde(rename = "Y")]
    y: f32,
    #[serde(rename = "Rotation")]
    rotation: f32,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OtdRelativeSettings {
    #[serde(rename = "XSensitivity")]
    x_sens: f32,
    #[serde(rename = "YSensitivity")]
    y_sens: f32,
    #[serde(rename = "RelativeRotation")]
    rotation: f32,
}

/// Imports an `OpenTabletDriver` configuration or preset file into a `NextTabletDriver` `MappingConfig`.
/// It converts OTD's Center-based Display coordinates to `NextTabletDriver`'s Top-Left coordinates.
///
/// # Errors
/// Returns an error string if the file could not be read or parsed.
pub fn import_otd_profile(path: &Path) -> Result<MappingConfig, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read OTD settings: {e}"))?;
    let otd: OtdSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse OTD settings: {e}"))?;

    let profile = otd
        .profiles
        .first()
        .ok_or("No profiles found in OTD settings")?;

    let mut config = MappingConfig::default();

    if let Some(out) = &profile.output_mode {
        if out.path.contains("AbsoluteMode") {
            config.mode = DriverMode::Absolute;
        } else if out.path.contains("RelativeMode") {
            config.mode = DriverMode::Relative;
        }
    }

    if let Some(abs) = &profile.absolute_settings {
        config.active_area = ActiveArea {
            w: abs.tablet.width,
            h: abs.tablet.height,
            x: abs.tablet.x,
            y: abs.tablet.y,
            rotation: abs.tablet.rotation,
        };

        config.target_area = TargetArea {
            w: abs.display.width,
            h: abs.display.height,
            x: abs.display.x - (abs.display.width / 2.0),
            y: abs.display.y - (abs.display.height / 2.0),
        };
    }

    if let Some(rel) = &profile.relative_settings {
        config.relative_config = RelativeConfig {
            x_sensitivity: rel.x_sens,
            y_sensitivity: rel.y_sens,
            rotation: rel.rotation,
            reset_time_ms: 100, // OTD uses a string TimeSpan like "00:00:00.1000000", fallback to default 100ms
        };
    }

    // Validate and repair will handle cases where fields were missing and defaulted to 0.0
    config.validate_and_repair();

    log::info!(target: "Config", "Successfully imported OTD profile from {:?}", path.file_name().unwrap_or_default().display());
    Ok(config)
}
