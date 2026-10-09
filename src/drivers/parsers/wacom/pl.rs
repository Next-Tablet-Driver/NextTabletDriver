use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;
use crate::engine::state::{LockRecoveryExt, WriteRecoverExt};
use std::sync::Mutex;

pub struct PLParser {
    initial_eraser: Mutex<bool>,
    last_report_out_of_range: Mutex<bool>,
}

impl PLParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            initial_eraser: Mutex::new(false),
            last_report_out_of_range: Mutex::new(true),
        }
    }
}

impl Default for PLParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for PLParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, b1, b2, b3, b4, b5, b6, b7, ..] => {
                if (*b1 & 0x40) == 0 {
                    *self
                        .last_report_out_of_range
                        .lock()
                        .unwrap_or_reset("wacom_pl_out_of_range") = true;
                    return None;
                }

                let mut out_of_range_guard = self
                    .last_report_out_of_range
                    .lock()
                    .unwrap_or_reset("wacom_pl_out_of_range");
                if *out_of_range_guard {
                    *self
                        .initial_eraser
                        .lock()
                        .unwrap_or_reset("wacom_pl_eraser") = (*b4 & 0x20) != 0;
                    *out_of_range_guard = false;
                    drop(out_of_range_guard);
                }

                let is_initial_eraser =
                    *self.initial_eraser.lock().unwrap_or_log("wacom_pl_eraser");

                let x = u32::from(*b1 & 0x03) << 14 | u32::from(*b2) << 7 | u32::from(*b3);
                let y = u32::from(*b4 & 0x03) << 14 | u32::from(*b5) << 7 | u32::from(*b6);

                let pressure = u32::from(*b7 ^ 0x40) << 2
                    | u32::from(*b4 & 0x40) >> 5
                    | u32::from(*b4 & 0x04) >> 2;

                let mut buttons: u8 = 0;
                if (*b4 & 0x10) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b4 & 0x20) != 0 && !is_initial_eraser {
                    buttons |= 1 << 1;
                }

                let eraser = (*b4 & 0x20) != 0 && is_initial_eraser;
                let status = if pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else {
                    crate::drivers::TabletStatus::Hover
                };

                let mut tablet_data = TabletData {
                    status,
                    x,
                    y,
                    pressure: pressure as u16,
                    buttons,
                    eraser,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            _ => None,
        }
    }
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
    use crate::drivers::TabletStatus;

    /// `[_, b1..b7]`: only the seven data bytes matter, the first one is ignored.
    fn report(b: [u8; 7]) -> [u8; 8] {
        [0x00, b[0], b[1], b[2], b[3], b[4], b[5], b[6]]
    }

    #[test]
    fn decodes_seven_bit_packed_coordinates_and_pressure() {
        let parser = PLParser::new();
        // x = (1 << 14) | (2 << 7) | 3, y = (2 << 14) | (1 << 7) | 5
        let data = report([0x41, 0x02, 0x03, 0x46, 0x01, 0x05, 0x41]);
        let parsed = parser.parse(&data).unwrap();
        assert_eq!(parsed.x, 16384 + 256 + 3);
        assert_eq!(parsed.y, 32768 + 128 + 5);
        // (0x41 ^ 0x40) << 2 | (0x46 & 0x40) >> 5 | (0x46 & 0x04) >> 2
        assert_eq!(parsed.pressure, 4 + 2 + 1);
        assert_eq!(parsed.status, TabletStatus::Contact);
    }

    #[test]
    fn zero_pressure_is_hover() {
        let parser = PLParser::new();
        let parsed = parser
            .parse(&report([0x40, 0, 0, 0x00, 0, 0, 0x40]))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Hover);
        assert_eq!(parsed.pressure, 0);
    }

    #[test]
    fn reports_without_the_proximity_bit_are_dropped() {
        let parser = PLParser::new();
        assert!(parser.parse(&report([0x01, 2, 3, 4, 5, 6, 7])).is_none());
    }

    #[test]
    fn the_eraser_end_is_latched_when_the_pen_enters_range() {
        let parser = PLParser::new();
        // Entering range with the eraser bit set: it is the eraser tip, not a barrel button.
        let first = parser
            .parse(&report([0x40, 0, 0, 0x20, 0, 0, 0x40]))
            .unwrap();
        assert!(first.eraser);
        assert_eq!(first.buttons, 0);

        // Leaving range resets the latch: the next entry is evaluated again.
        assert!(parser.parse(&report([0x00; 7])).is_none());
        let tip = parser
            .parse(&report([0x40, 0, 0, 0x00, 0, 0, 0x40]))
            .unwrap();
        assert!(!tip.eraser);

        // Same bit while the pen entered as a tip: it is the second barrel button.
        let barrel = parser
            .parse(&report([0x40, 0, 0, 0x30, 0, 0, 0x40]))
            .unwrap();
        assert!(!barrel.eraser);
        assert_eq!(barrel.buttons, 0b11);
    }

    #[test]
    fn rejects_short_reports() {
        assert!(PLParser::default().parse(&[0x00, 0x40, 0, 0]).is_none());
    }
}
