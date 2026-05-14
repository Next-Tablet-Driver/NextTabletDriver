use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BambooPadParser;

impl ReportParser for BambooPadParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

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

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x,
                    y,
                    pressure,
                    buttons,
                    eraser,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
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

                Some(TabletData {
                    status: "Aux".to_string(),
                    buttons,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}
