//! # Generic Tablet Driver
//!
//! This module implements a generic driver layer that unifies the interaction
//! with all supported tablet models. It reads their JSON configurations, routes
//! initialization patterns, and instantiates the correct specific data parser.

use super::config::{DigitizerIdentifier, TabletConfiguration};
use super::parsers::{ReportParser, create_parser};
use super::{NextTabletDriver, TabletData};

/// A universal wrapper implementing the `NextTabletDriver` trait.
///
/// Instead of writing a different `Driver` struct for every single tablet model,
/// this generic struct uses the loaded `TabletConfiguration` to dynamically answer
/// questions about its specs and routes the raw USB byte array `parse()` calls
/// to the specific sub-parser (Wacom, Huion, XP-Pen, etc.) defined in the config.
pub struct GenericNextTabletDriver {
    config: TabletConfiguration,
    #[allow(dead_code)]
    digitizer: Option<DigitizerIdentifier>,
    vid: u16,
    pid: u16,
    parser: Box<dyn ReportParser>,
}

impl GenericNextTabletDriver {
    #[must_use]
    pub fn new(
        config: TabletConfiguration,
        digitizer: &DigitizerIdentifier,
        vid: u16,
        pid: u16,
    ) -> Self {
        let parser_name = digitizer.report_parser.as_str();

        let parser = create_parser(parser_name);

        Self {
            config,
            digitizer: Some(digitizer.clone()),
            vid,
            pid,
            parser,
        }
    }
}

impl NextTabletDriver for GenericNextTabletDriver {
    fn get_name(&self) -> &str {
        &self.config.name
    }

    fn get_specs(&self) -> (f32, f32, f32) {
        (
            self.config.specifications.digitizer.max_x,
            self.config.specifications.digitizer.max_y,
            f32::from(self.config.specifications.pen.max_pressure),
        )
    }

    fn get_physical_specs(&self) -> (f32, f32) {
        (
            self.config.specifications.digitizer.width,
            self.config.specifications.digitizer.height,
        )
    }

    fn get_vid_pid(&self) -> (u16, u16) {
        (self.vid, self.pid)
    }

    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        #[cfg(target_os = "linux")]
        {
            // Matches the fixed-size HID read buffer in `tablet_manager.rs`;
            // reports are never larger than this in practice.
            const MAX_REPORT_LEN: usize = 64;

            if let Some(ref d) = self.digitizer
                && let Some(expected_len) = d.input_report_length
                && data.len() == expected_len
                && data.len() <= MAX_REPORT_LEN
            {
                // Prefix with a 0x00 report-ID byte on the stack instead of heap-allocating
                // a Vec for every single HID report.
                let total_len = data.len() + 1;
                let mut buf = [0u8; MAX_REPORT_LEN + 1];
                if let Some(dest) = buf.get_mut(1..total_len) {
                    dest.copy_from_slice(data);
                    if let Some(report) = buf.get(..total_len) {
                        return self.parser.parse(report);
                    }
                }
            }
        }
        self.parser.parse(data)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    const CONFIG: &str = r#"{
        "Name": "Test Tablet",
        "Specifications": {
            "Digitizer": { "Width": 160.0, "Height": 100.0, "MaxX": 16000, "MaxY": 10000 },
            "Pen": { "MaxPressure": 8191, "ButtonCount": 2 }
        },
        "DigitizerIdentifiers": [
            { "VendorID": 4660, "ProductID": 22136, "ReportParser": "Test.Tablet.ReportParser" }
        ]
    }"#;

    fn driver(input_report_length: Option<usize>) -> GenericNextTabletDriver {
        let mut config: TabletConfiguration = serde_json::from_str(CONFIG).unwrap();
        config.digitizer_identifiers[0].input_report_length = input_report_length;
        let digitizer = config.digitizer_identifiers[0].clone();
        GenericNextTabletDriver::new(config, &digitizer, 0x1234, 0x5678)
    }

    #[test]
    fn exposes_the_configuration_as_driver_specs() {
        let driver = driver(None);
        assert_eq!(driver.get_name(), "Test Tablet");
        assert_eq!(driver.get_specs(), (16000.0, 10000.0, 8191.0));
        assert_eq!(driver.get_physical_specs(), (160.0, 100.0));
        assert_eq!(driver.get_vid_pid(), (0x1234, 0x5678));
    }

    #[test]
    fn reports_go_through_the_parser_named_in_the_configuration() {
        // "Tablet" in the parser name selects the generic fallback layout.
        let parsed = driver(None)
            .parse(&[0x02, 0x01, 0x10, 0x00, 0x20, 0x00, 0x05, 0x00])
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!((parsed.x, parsed.y, parsed.pressure), (16, 32, 5));
        assert!(driver(None).parse(&[0x02]).is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_prefixes_reports_of_the_expected_length_with_a_zero_report_id() {
        // hidraw omits the report id, so a report of exactly `InputReportLength` bytes gets a
        // leading 0x00 before reaching the parser: the status byte is then the first byte.
        let data = [0xA1, 0x10, 0x00, 0x20, 0x00, 0x05, 0x00, 0x00];
        let parsed = driver(Some(8)).parse(&data).unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!((parsed.x, parsed.y, parsed.pressure), (16, 32, 5));
        assert_eq!(parsed.raw_len, 9);
    }
}
