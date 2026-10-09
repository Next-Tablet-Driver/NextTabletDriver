//! # Tablet Configuration Serialization
//!
//! This module defines the Serde structures used to parse the standard `.json`
//! configuration files (compatible with `OpenTabletDriver` format). These files contain
//! the physical specifications, USB identifiers, and initialization sequences for
//! hundreds of known tablet models.

use serde::Deserialize;
use std::collections::HashMap;

/// The root structure representing a parsed tablet configuration file.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct TabletConfiguration {
    /// The human-readable name of the tablet (e.g., "Wacom CTL-472").
    pub name: String,
    /// Physical dimensions and hardware limits.
    pub specifications: Specifications,
    /// USB/Bluetooth matching criteria and initialization payloads.
    pub digitizer_identifiers: Vec<DigitizerIdentifier>,
    /// Optional overrides for specific operating systems/backends.
    pub attributes: Option<Attributes>,
}

/// Groups the hardware capabilities of the tablet body and the stylus.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Specifications {
    pub digitizer: DigitizerSpecs,
    pub pen: PenSpecs,
}

/// Details the mapping area size and maximum raw values the HID descriptor can emit.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DigitizerSpecs {
    /// Physical width in millimeters.
    pub width: f32,
    /// Physical height in millimeters.
    pub height: f32,
    /// Maximum `X` value emitted by the hardware sensor.
    pub max_x: f32,
    /// Maximum `Y` value emitted by the hardware sensor.
    pub max_y: f32,
}

/// Details the capabilities of the specific stylus included with the tablet.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PenSpecs {
    /// The maximum pressure level (e.g., 2047, 4095, 8191).
    pub max_pressure: u16,
    /// Number of physical buttons on the pen barrel.
    pub button_count: Option<u8>,
}

/// Connects specific USB Hardware IDs to the data parser required to understand them.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DigitizerIdentifier {
    /// USB Vendor ID (e.g., 0x056A for Wacom).
    #[serde(rename = "VendorID")]
    pub vendor_id: u16,
    /// USB Product ID (e.g., 0x0374).
    #[serde(rename = "ProductID")]
    pub product_id: u16,
    /// Expected byte length of incoming HID packets.
    pub input_report_length: Option<usize>,
    pub output_report_length: Option<usize>,
    /// Regex patterns keyed by HID string-descriptor index (e.g. `201` for
    /// the firmware-version string), used to disambiguate configs that
    /// share the same `VendorID`/`ProductID`/`InputReportLength` — mirrors
    /// `OpenTabletDriver`'s `Driver.DeviceMatchesStrings` in `Driver.cs`.
    pub device_strings: Option<HashMap<u8, String>>,
    /// The fully qualified C# class name from `OpenTabletDriver` format mapping to our Rust parsers.
    pub report_parser: String,
    /// Base64 encoded byte array(s) to send via `Device::write` to wake up the tablet.
    pub output_init_report: Option<Vec<String>>,
    /// Base64 encoded byte array(s) to send via `Device::send_feature_report` to put the tablet in Absolute/Pro mode.
    pub feature_init_report: Option<Vec<String>>,
    pub attributes: Option<Attributes>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Attributes {
    pub libinputoverride: Option<String>,
    #[serde(rename = "Interface")]
    pub interface: Option<serde_json::Value>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_g640() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
  "Name": "XP-Pen Star G640 (Variant 2)",
  "Specifications": {
    "Digitizer": {
      "Width": 159.99,
      "Height": 99.99,
      "MaxX": 15999,
      "MaxY": 9999
    },
    "Pen": {
      "MaxPressure": 8191,
      "ButtonCount": 2
    }
  },
  "DigitizerIdentifiers": [
    {
      "VendorID": 10429,
      "ProductID": 2324,
      "InputReportLength": 12,
      "OutputReportLength": 12,
      "ReportParser": "OpenTabletDriver.Configurations.Parsers.XP_Pen.XP_PenReportParser",
      "OutputInitReport": [
        "ArAE"
      ]
    }
  ],
  "Attributes": {
    "libinputoverride": "1"
  }
}"#;

        let config: TabletConfiguration = serde_json::from_str(json)?;
        assert_eq!(config.name, "XP-Pen Star G640 (Variant 2)");
        assert_eq!(config.digitizer_identifiers[0].vendor_id, 10429);
        assert_eq!(config.specifications.digitizer.width, 159.99);
        Ok(())
    }

    #[test]
    fn test_deserialize_ctl472() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
  "Name": "Wacom CTL-472",
  "Specifications": {
    "Digitizer": {
      "Width": 152,
      "Height": 95,
      "MaxX": 15200,
      "MaxY": 9500
    },
    "Pen": {
      "MaxPressure": 2047,
      "ButtonCount": 2
    }
  },
  "DigitizerIdentifiers": [
    {
      "VendorID": 1386,
      "ProductID": 890,
      "InputReportLength": 10,
      "ReportParser": "OpenTabletDriver.Configurations.Parsers.Wacom.Intuos.IntuosReportParser",
      "FeatureInitReport": [
        "AgI="
      ]
    }
  ],
  "Attributes": {}
}"#;

        let config: TabletConfiguration = serde_json::from_str(json)?;
        assert_eq!(config.name, "Wacom CTL-472");
        assert_eq!(
            config.digitizer_identifiers[0]
                .feature_init_report
                .as_ref()
                .ok_or("Missing feature_init_report")?[0],
            "AgI="
        );
        Ok(())
    }

    #[test]
    fn test_deserialize_device_strings() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
  "Name": "Gaomon S620",
  "Specifications": {
    "Digitizer": {
      "Width": 165.1,
      "Height": 101.6,
      "MaxX": 33020,
      "MaxY": 20320
    },
    "Pen": {
      "MaxPressure": 8191,
      "ButtonCount": 2
    }
  },
  "DigitizerIdentifiers": [
    {
      "VendorID": 9580,
      "ProductID": 111,
      "InputReportLength": 12,
      "ReportParser": "OpenTabletDriver.Configurations.Parsers.UCLogic.UCLogicReportParser",
      "DeviceStrings": {
        "201": "OEM02_T18e_\\d{6}$"
      }
    }
  ],
  "Attributes": {}
}"#;

        let config: TabletConfiguration = serde_json::from_str(json)?;
        let device_strings = config.digitizer_identifiers[0]
            .device_strings
            .as_ref()
            .ok_or("Missing device_strings")?;
        assert_eq!(
            device_strings.get(&201).map(String::as_str),
            Some("OEM02_T18e_\\d{6}$")
        );
        Ok(())
    }
}
