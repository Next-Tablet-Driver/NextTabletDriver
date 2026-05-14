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

        let status = match data.get(1) {
            Some(0xA0) => crate::drivers::TabletStatus::Hover,
            Some(0xA1) => crate::drivers::TabletStatus::Contact,
            Some(0xC0 | 0x00) => crate::drivers::TabletStatus::OutOfRange,
            Some(b1) => {
                if (b1 & 0x80) != 0 {
                    crate::drivers::TabletStatus::OutOfRange
                } else {
                    crate::drivers::TabletStatus::Active
                }
            }
            None => crate::drivers::TabletStatus::Disconnected,
        };

        let is_connected = status != crate::drivers::TabletStatus::OutOfRange;

        let mut tablet_data = TabletData {
            status,
            x,
            y,
            pressure,
            tilt_x,
            tilt_y,
            buttons,
            eraser,
            hover_distance: 0, // Not provided in this report
            is_connected,
            ..Default::default()
        };
        tablet_data.set_raw(data);

        Some(tablet_data)
    }
}
