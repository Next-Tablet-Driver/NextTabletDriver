use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct PTUParser;

impl ReportParser for PTUParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x10) != 0 {
                    buttons |= 1 << 1;
                }

                let eraser = (*b1 & 0x04) != 0;

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
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    fn report(b1: u8, x: u16, y: u16, pressure: u16) -> [u8; 8] {
        let (x, y, p) = (x.to_le_bytes(), y.to_le_bytes(), pressure.to_le_bytes());
        [0x02, b1, x[0], x[1], y[0], y[1], p[0], p[1]]
    }

    #[test]
    fn decodes_position_pressure_buttons_and_eraser() {
        let parsed = PTUParser
            .parse(&report(0x02 | 0x10 | 0x04, 0x1234, 0x5678, 0x03FF))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!(
            (parsed.x, parsed.y, parsed.pressure),
            (0x1234, 0x5678, 0x03FF)
        );
        assert_eq!(parsed.buttons, 0b11);
        assert!(parsed.eraser);
    }

    #[test]
    fn no_pressure_is_hover() {
        let parsed = PTUParser.parse(&report(0, 1, 2, 0)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Hover);
        assert_eq!(parsed.buttons, 0);
        assert!(!parsed.eraser);
    }

    #[test]
    fn rejects_other_ids_and_short_reports() {
        let mut data = report(0, 1, 2, 3);
        data[0] = 0x03;
        assert!(PTUParser.parse(&data).is_none());
        assert!(PTUParser.parse(&data[..7]).is_none());
    }
}
