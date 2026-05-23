use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BambooPadParser;

impl ReportParser for BambooPadParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            // Tablet Report
            [0x10, 0x01, b2, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*b2 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                let eraser = (*b2 & 0x08) != 0;

                let status = if pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else {
                    crate::drivers::TabletStatus::Hover
                };

                let mut tablet_data = TabletData {
                    status,
                    x,
                    y,
                    pressure,
                    buttons,
                    eraser,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            // Aux Report - needs to check index 23
            [0x10, 0x06, .., b23] if data.len() >= 24 => {
                let mut buttons: u8 = 0;
                if *b23 == 1 {
                    buttons |= 1 << 0;
                }
                if *b23 == 2 {
                    buttons |= 1 << 1;
                }

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons,
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
