use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct XpPenStarG640Parser;

impl ReportParser for XpPenStarG640Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        if data.len() < 8 {
            return None;
        }

        // Use pattern matching to safely extract the fixed-size fields
        let (x, y, pressure, buttons, eraser) = match data {
            [_, b1, x_low, x_high, y_low, y_high, p_low, p_high, ..] => {
                let x = (u16::from(*x_high) << 8) | u16::from(*x_low);
                let y = (u16::from(*y_high) << 8) | u16::from(*y_low);
                let p = (u16::from(*p_high) << 8) | u16::from(*p_low);
                let buttons = (*b1 >> 1) & 0x03;
                let eraser = (*b1 & 0x08) != 0;
                (x, y, p, buttons, eraser)
            }
            _ => return None,
        };

        // Tilt (X at 8, Y at 9) - optional/dynamic
        let tilt_x = data.get(8).copied().unwrap_or(0).cast_signed();
        let tilt_y = data.get(9).copied().unwrap_or(0).cast_signed();

        // Raw hex string for debugging
        let raw = data
            .iter()
            .take(14)
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        let status = match data.get(1) {
            Some(0xA0) => "Hover",
            Some(0xA1) => "Contact",
            Some(0xC0 | 0x00) => "Out of Range",
            Some(b1) => {
                if (b1 & 0x80) != 0 {
                    "Out of Range"
                } else {
                    "Active"
                }
            }
            None => "Disconnected",
        }
        .to_string();

        let is_connected = status != "Out of Range";

        Some(TabletData {
            status,
            x,
            y,
            pressure,
            tilt_x,
            tilt_y,
            buttons,
            eraser,
            hover_distance: 0, // Not provided in this report
            raw_data: raw,
            is_connected,
            ..Default::default()
        })
    }
}
