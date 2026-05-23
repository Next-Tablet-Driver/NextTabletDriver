use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct XenceLabsParser;

impl ReportParser for XenceLabsParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            // Aux Report
            [_, report_byte, b2, ..] if (*report_byte & 0xF0) == 0xF0 => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: *b2,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            // Tablet Report
            [
                _,
                report_byte,
                x_lo,
                x_hi,
                y_lo,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                t_y,
                ..,
            ] if (*report_byte & 0x20) != 0 => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*report_byte & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*report_byte & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                if (*report_byte & 0x08) != 0 {
                    buttons |= 1 << 2;
                }

                let eraser = (*report_byte & 0x40) != 0;

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
                    tilt_x: t_x.cast_signed(),
                    tilt_y: t_y.cast_signed(),
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn test_xencelabs_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = XenceLabsParser;
        let data: [u8; 10] = [0, 0x2E, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00, 10, 20];
        let report = parser
            .parse(&data)
            .ok_or("XenceLabs parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 7);
        assert_eq!(report.tilt_x, 10);
        Ok(())
    }
}
