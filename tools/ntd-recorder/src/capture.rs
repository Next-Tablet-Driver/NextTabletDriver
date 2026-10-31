//! The `.ntdcap` file: a versioned JSON capture of one tablet session.
//!
//! The file holds no personal data: no HID path (it embeds the machine's USB topology), no machine
//! name, no user name, and only the last characters of the serial number.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::path::Path;

/// Name of the format, written in every file so other tools can recognise it.
pub const FORMAT: &str = "ntdcap";
/// Version of the layout below. Bump it on any incompatible change.
pub const FORMAT_VERSION: u32 = 1;

/// How many trailing characters of the serial number are kept.
const SERIAL_HINT_LEN: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capture {
    pub format: String,
    pub format_version: u32,
    pub tool_version: String,
    pub device: DeviceInfo,
    pub interfaces: Vec<InterfaceInfo>,
    pub steps: Vec<StepInfo>,
    pub reports: Vec<Report>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    /// `…` followed by the last characters of the serial number, never the whole serial.
    pub serial_hint: Option<String>,
    pub release_number: u16,
}

/// One HID interface (collection) of the device. Tablets usually expose several.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceInfo {
    /// Position in this file; `Report::interface` refers to it.
    pub index: usize,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    /// HID report descriptor as lowercase hexadecimal, `None` when the OS did not provide it.
    pub descriptor_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepInfo {
    pub id: String,
    pub instruction: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// Milliseconds since the start of the session.
    pub t_ms: u64,
    pub interface: usize,
    /// Index into `Capture::steps`.
    pub step: usize,
    pub data_hex: String,
}

/// Lowercase hexadecimal encoding of `bytes`.
#[must_use]
pub fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Keeps only the end of a serial number, enough to tell two tablets apart but not to identify one.
#[must_use]
pub fn serial_hint(serial: &str) -> Option<String> {
    let serial = serial.trim();
    if serial.is_empty() {
        return None;
    }
    let skip = serial.chars().count().saturating_sub(SERIAL_HINT_LEN);
    Some(format!(
        "…{}",
        serial.chars().skip(skip).collect::<String>()
    ))
}

impl Capture {
    #[must_use]
    pub fn new(device: DeviceInfo) -> Self {
        Self {
            format: FORMAT.to_owned(),
            format_version: FORMAT_VERSION,
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            device,
            interfaces: Vec::new(),
            steps: Vec::new(),
            reports: Vec::new(),
        }
    }

    /// Writes the capture as pretty JSON.
    ///
    /// # Errors
    /// Returns an error when the file cannot be written.
    pub fn write(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Reads a capture and rejects files of another format or a newer version.
    ///
    /// # Errors
    /// Returns an error when the file is unreadable, is not JSON, or is not a supported capture.
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let capture: Self = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        if capture.format != FORMAT {
            return Err(format!("not a {FORMAT} file (format = {:?})", capture.format).into());
        }
        if capture.format_version > FORMAT_VERSION {
            return Err(format!(
                "unsupported {FORMAT} version {} (this tool reads up to {FORMAT_VERSION})",
                capture.format_version
            )
            .into());
        }
        Ok(capture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> DeviceInfo {
        DeviceInfo {
            vendor_id: 0x28bd,
            product_id: 0x0914,
            manufacturer: Some("XP-Pen".into()),
            product: Some("Deco 01 V2".into()),
            serial_hint: serial_hint("ABCDEF123456"),
            release_number: 0x0100,
        }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ntd-recorder-{}-{name}", std::process::id()))
    }

    #[test]
    fn hex_is_lowercase_and_zero_padded() {
        assert_eq!(to_hex(&[0x00, 0x0a, 0xff]), "000aff");
        assert_eq!(to_hex(&[]), "");
    }

    #[test]
    fn a_serial_number_is_cut_to_its_last_characters() {
        assert_eq!(serial_hint("ABCDEF123456").as_deref(), Some("…3456"));
        assert_eq!(serial_hint("12").as_deref(), Some("…12"));
    }

    #[test]
    fn an_empty_serial_number_gives_no_hint() {
        assert_eq!(serial_hint(""), None);
        assert_eq!(serial_hint("   "), None);
    }

    #[test]
    fn the_serial_hint_never_contains_the_whole_serial() {
        let hint = serial_hint("SECRETSERIAL9999").unwrap_or_default();
        assert!(!hint.contains("SECRET"));
    }

    #[test]
    fn a_capture_survives_a_round_trip_through_a_file() {
        let mut capture = Capture::new(device());
        capture.interfaces.push(InterfaceInfo {
            index: 0,
            interface_number: 1,
            usage_page: 0x0d,
            usage: 0x02,
            descriptor_hex: Some("05010902".into()),
        });
        capture.steps.push(StepInfo {
            id: "hover".into(),
            instruction: "Hover".into(),
            start_ms: 0,
            end_ms: 5000,
        });
        capture.reports.push(Report {
            t_ms: 12,
            interface: 0,
            step: 0,
            data_hex: "0801020304".into(),
        });
        let path = temp("roundtrip.ntdcap");
        assert!(capture.write(&path).is_ok());
        let read = Capture::read(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(read.ok(), Some(capture));
    }

    #[test]
    fn a_file_of_another_format_is_rejected() {
        let mut capture = Capture::new(device());
        capture.format = "something-else".into();
        let path = temp("wrong-format.ntdcap");
        assert!(capture.write(&path).is_ok());
        let read = Capture::read(&path);
        let _ = std::fs::remove_file(&path);
        assert!(read.is_err());
    }

    #[test]
    fn a_newer_version_is_rejected() {
        let mut capture = Capture::new(device());
        capture.format_version = FORMAT_VERSION + 1;
        let path = temp("newer.ntdcap");
        assert!(capture.write(&path).is_ok());
        let read = Capture::read(&path);
        let _ = std::fs::remove_file(&path);
        assert!(read.is_err());
    }

    #[test]
    fn the_file_holds_no_path_or_machine_field() {
        let json = serde_json::to_string(&Capture::new(device())).unwrap_or_default();
        for forbidden in ["path", "hostname", "user", "serial_number"] {
            assert!(!json.contains(forbidden), "unexpected field {forbidden}");
        }
    }
}
