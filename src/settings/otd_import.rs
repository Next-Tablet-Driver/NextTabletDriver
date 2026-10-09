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

/// Parses an `OpenTabletDriver` configuration string into a `NextTabletDriver` `MappingConfig`.
/// It converts OTD's Center-based Display coordinates to `NextTabletDriver`'s Top-Left coordinates.
///
/// # Errors
/// Returns an error string if the JSON string cannot be parsed or contains no profiles.
pub fn parse_otd_profile_str(content: &str) -> Result<MappingConfig, String> {
    let otd: OtdSettings =
        serde_json::from_str(content).map_err(|e| format!("Failed to parse OTD settings: {e}"))?;

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
    Ok(config)
}

/// Imports an `OpenTabletDriver` configuration or preset file into a `NextTabletDriver` `MappingConfig`.
///
/// # Errors
/// Returns an error string if the file could not be read or parsed.
pub fn import_otd_profile(path: &Path) -> Result<MappingConfig, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read OTD settings: {e}"))?;
    let config = parse_otd_profile_str(&content)?;
    log::info!(target: "Config", "Successfully imported OTD profile from {:?}", path.file_name().unwrap_or_default().display());
    Ok(config)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_import_otd_absolute_mode_recentering() {
        let json = r#"{
            "Profiles": [
                {
                    "OutputMode": {
                        "Path": "OpenTabletDriver.Desktop.Profiles.AbsoluteMode"
                    },
                    "AbsoluteModeSettings": {
                        "Display": {
                            "Width": 1920.0,
                            "Height": 1080.0,
                            "X": 960.0,
                            "Y": 540.0,
                            "Rotation": 0.0
                        },
                        "Tablet": {
                            "Width": 120.0,
                            "Height": 80.0,
                            "X": 60.0,
                            "Y": 40.0,
                            "Rotation": 15.0
                        }
                    }
                }
            ]
        }"#;

        let config = parse_otd_profile_str(json).expect("Failed to parse valid OTD JSON");
        assert_eq!(config.mode, DriverMode::Absolute);

        // Center-based Display (960, 540) with 1920x1080 -> Top-Left should be (0.0, 0.0)
        assert!((config.target_area.x - 0.0).abs() < 1e-5);
        assert!((config.target_area.y - 0.0).abs() < 1e-5);
        assert!((config.target_area.w - 1920.0).abs() < 1e-5);
        assert!((config.target_area.h - 1080.0).abs() < 1e-5);

        // Tablet Area
        assert!((config.active_area.x - 60.0).abs() < 1e-5);
        assert!((config.active_area.y - 40.0).abs() < 1e-5);
        assert!((config.active_area.w - 120.0).abs() < 1e-5);
        assert!((config.active_area.h - 80.0).abs() < 1e-5);
        assert!((config.active_area.rotation - 15.0).abs() < 1e-5);
    }

    #[test]
    fn test_import_otd_relative_mode() {
        let json = r#"{
            "Profiles": [
                {
                    "OutputMode": {
                        "Path": "OpenTabletDriver.Desktop.Profiles.RelativeMode"
                    },
                    "RelativeModeSettings": {
                        "XSensitivity": 2.5,
                        "YSensitivity": 3.2,
                        "RelativeRotation": 45.0
                    }
                }
            ]
        }"#;

        let config = parse_otd_profile_str(json).expect("Failed to parse relative OTD JSON");
        assert_eq!(config.mode, DriverMode::Relative);
        assert!((config.relative_config.x_sensitivity - 2.5).abs() < 1e-5);
        assert!((config.relative_config.y_sensitivity - 3.2).abs() < 1e-5);
        assert!((config.relative_config.rotation - 45.0).abs() < 1e-5);
        assert_eq!(config.relative_config.reset_time_ms, 100);
    }

    #[test]
    fn test_import_otd_errors() {
        // Empty profiles
        let empty_json = r#"{ "Profiles": [] }"#;
        assert!(parse_otd_profile_str(empty_json).is_err());

        // Malformed JSON
        let malformed = r#"{ "Profiles": [ invalid json ] }"#;
        assert!(parse_otd_profile_str(malformed).is_err());
    }

    #[test]
    fn test_import_otd_missing_fields_repaired() {
        // Missing dimensions default to 0 in serde, but validate_and_repair should fix them
        let partial_json = r#"{
            "Profiles": [
                {
                    "OutputMode": {
                        "Path": "OpenTabletDriver.Desktop.Profiles.AbsoluteMode"
                    }
                }
            ]
        }"#;

        let config =
            parse_otd_profile_str(partial_json).expect("Partial profile should be repaired");
        assert_eq!(config.mode, DriverMode::Absolute);
        assert!(config.active_area.w > 0.0);
        assert!(config.active_area.h > 0.0);
        assert!(config.target_area.w > 0.0);
        assert!(config.target_area.h > 0.0);
    }

    #[test]
    fn test_import_otd_from_file() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("test_otd_profile.json");

        let json = r#"{
            "Profiles": [
                {
                    "OutputMode": { "Path": "AbsoluteMode" },
                    "AbsoluteModeSettings": {
                        "Display": { "Width": 1920.0, "Height": 1080.0, "X": 960.0, "Y": 540.0, "Rotation": 0.0 },
                        "Tablet": { "Width": 100.0, "Height": 100.0, "X": 50.0, "Y": 50.0, "Rotation": 0.0 }
                    }
                }
            ]
        }"#;

        std::fs::write(&test_path, json).expect("Failed to write temp file");
        let result = import_otd_profile(&test_path);
        let _ = std::fs::remove_file(&test_path);

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.mode, DriverMode::Absolute);
        assert!((config.active_area.w - 100.0).abs() < 1e-5);
    }

    #[test]
    fn an_unrecognised_output_mode_keeps_the_default_mode() {
        let json = r#"{ "Profiles": [ { "OutputMode": { "Path": "Some.Other.Mode" } } ] }"#;
        let config = parse_otd_profile_str(json).unwrap();
        assert_eq!(config.mode, MappingConfig::default().mode);
    }

    #[test]
    fn a_profile_without_an_output_mode_keeps_the_default_mode() {
        let config = parse_otd_profile_str(r#"{ "Profiles": [ {} ] }"#).unwrap();
        assert_eq!(config.mode, MappingConfig::default().mode);
    }

    #[test]
    fn a_file_that_does_not_exist_cannot_be_imported() {
        let error = import_otd_profile(Path::new("ntd-no-such-otd-settings.json")).unwrap_err();
        assert!(error.starts_with("Failed to read OTD settings"), "{error}");
    }

    #[test]
    fn an_otd_file_on_disk_is_imported() {
        log::set_max_level(log::LevelFilter::Trace);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ntd_otd_import_{nanos}.json"));
        fs::write(
            &path,
            r#"{ "Profiles": [ { "OutputMode": { "Path": "OpenTabletDriver.Desktop.Output.RelativeMode" } } ] }"#,
        )
        .unwrap();
        let config = import_otd_profile(&path).unwrap();
        assert_eq!(config.mode, DriverMode::Relative);
        let _ = fs::remove_file(path);
    }
}
