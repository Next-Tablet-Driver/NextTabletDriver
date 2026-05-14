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
        if data.len() < 8 {
            return None;
        }

        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        if (data[1] & 0x40) == 0 {
            *self.last_report_out_of_range.lock().unwrap_or_reset("wacom_pl_out_of_range") = true;
            return None; // OutOfRangeReport logic doesn't carry in TabletData struct, mapping to Option::None
        }

        let mut out_of_range_guard = self.last_report_out_of_range.lock().unwrap_or_reset("wacom_pl_out_of_range");
        if *out_of_range_guard {
            *self.initial_eraser.lock().unwrap_or_reset("wacom_pl_eraser") = (data[4] & 0x20) != 0;
            *out_of_range_guard = false;
        }

        let is_initial_eraser = *self.initial_eraser.lock().unwrap_or_log("wacom_pl_eraser");

        let x_part1 = ((data[1] & 0x03) as u32) << 14;
        let x_part2 = (data[2] as u32) << 7;
        let x_part3 = data[3] as u32;
        let x = x_part1 + x_part2 + x_part3;

        let y_part1 = ((data[4] & 0x03) as u32) << 14;
        let y_part2 = (data[5] as u32) << 7;
        let y_part3 = data[6] as u32;
        let y = y_part1 + y_part2 + y_part3;

        let pressure_part1 = ((data[7] ^ 0x40) as u32) << 2;
        let pressure_part2 = ((data[4] & 0x40) as u32) >> 5;
        let pressure_part3 = ((data[4] & 0x04) as u32) >> 2;
        let pressure = pressure_part1 + pressure_part2 + pressure_part3;

        let mut buttons: u8 = 0;
        if (data[4] & 0x10) != 0 {
            buttons |= 1 << 0;
        }
        if (data[4] & 0x20) != 0 && !is_initial_eraser {
            buttons |= 1 << 1;
        }

        let eraser = (data[4] & 0x20) != 0 && is_initial_eraser;
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
}
