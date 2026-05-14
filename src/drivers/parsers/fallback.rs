use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct FallbackParser;

impl ReportParser for FallbackParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .take(10)
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, status_byte, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let status = if *status_byte == 0xC0 || *status_byte == 0x00 {
                    "Out of Range".to_string()
                } else if (*status_byte & 0x01) != 0 || pressure > 0 {
                    "Contact".to_string()
                } else {
                    "Hover".to_string()
                };

                let is_connected = status != "Out of Range";

                Some(TabletData {
                    status,
                    x,
                    y,
                    pressure,
                    tilt_x: 0,
                    tilt_y: 0,
                    buttons: 0,
                    eraser: false,
                    hover_distance: 0,
                    raw_data: raw,
                    is_connected,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}
