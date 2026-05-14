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
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

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
                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x: x as u16,
                    y: y as u16,
                    pressure: pressure as u16,
                    buttons,
                    eraser,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}
