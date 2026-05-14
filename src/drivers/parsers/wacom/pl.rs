use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;
use crate::engine::state::{LockRecoveryExt, WriteRecoverExt};
use std::sync::Mutex;

pub struct PLParser {
    initial_eraser: Mutex<bool>,
    last_report_out_of_range: Mutex<bool>,
}

impl PLParser {
    pub fn new() -> Self {
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
            .map(|b| format!("{:02X}", b))
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
                }

                let is_initial_eraser =
                    *self.initial_eraser.lock().unwrap_or_log("wacom_pl_eraser");

                let x = ((*b1 & 0x03) as u32) << 14 | (*b2 as u32) << 7 | (*b3 as u32);
                let y = ((*b4 & 0x03) as u32) << 14 | (*b5 as u32) << 7 | (*b6 as u32);

                let pressure = ((*b7 ^ 0x40) as u32) << 2
                    | ((*b4 & 0x40) as u32) >> 5
                    | ((*b4 & 0x04) as u32) >> 2;

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
