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
                buf[1..total_len].copy_from_slice(data);
                return self.parser.parse(&buf[..total_len]);
            }
        }
        self.parser.parse(data)
    }
}
