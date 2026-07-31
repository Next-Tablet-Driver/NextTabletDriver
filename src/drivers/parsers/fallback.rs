use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct FallbackParser;

impl ReportParser for FallbackParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, status_byte, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let status = if *status_byte == 0xC0 || *status_byte == 0x00 {
                    crate::drivers::TabletStatus::OutOfRange
                } else if (*status_byte & 0x01) != 0 || pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else {
                    crate::drivers::TabletStatus::Hover
                };

                let is_connected = status != crate::drivers::TabletStatus::OutOfRange;

                let mut tablet_data = TabletData {
                    status,
                    x,
                    y,
                    pressure,
                    tilt_x: 0,
                    tilt_y: 0,
                    buttons: 0,
                    eraser: false,
                    hover_distance: 0,
                    is_connected,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            _ => None,
        }
    }
}
