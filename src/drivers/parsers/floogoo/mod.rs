use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct FlooGooParser;

impl ReportParser for FlooGooParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [
                0x01,
                b1,
                x_lo,
                x_hi,
                y_lo,
                y_hi,
                p_lo,
                p_hi,
                tx_lo,
                tx_hi,
                ty_lo,
                ty_hi,
                ..,
            ] if (*b1 & 0x20) != 0 => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let raw_tilt_x = i16::from_le_bytes([*tx_lo, *tx_hi]);
                let raw_tilt_y = i16::from_le_bytes([*ty_lo, *ty_hi]);
                let tilt_x = (f32::from(raw_tilt_x) * 0.01).round() as i8;
                let tilt_y = (f32::from(raw_tilt_y) * 0.01).round() as i8;

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }

                let eraser = (*b1 & 0x08) != 0;
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
                    tilt_x,
                    tilt_y,
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
mod tests {
    use super::*;

    #[test]
    fn test_floogoo_pen_contact() -> Result<(), Box<dyn std::error::Error>> {
        let parser = FlooGooParser;
        let data: [u8; 12] = [
            0x01, 0x2A, 0x02, 0x01, 0x04, 0x03, 0x05, 0x00, 0xE8, 0x03, 0x18, 0xFC,
        ];
        let report = parser
            .parse(&data)
            .ok_or("FlooGoo parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 5);
        assert_eq!(report.buttons, 1); // 0x02 is bit 0
        assert!(report.eraser); // 0x08 is eraser
        assert_eq!(report.tilt_x, 10);
        assert_eq!(report.tilt_y, -10);
        Ok(())
    }

    #[test]
    fn test_floogoo_out_of_range() {
        let parser = FlooGooParser;
        let data: [u8; 12] = [0x01, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(parser.parse(&data).is_none());
    }
}
