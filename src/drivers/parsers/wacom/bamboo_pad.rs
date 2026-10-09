use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BambooPadParser;

impl ReportParser for BambooPadParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            // Tablet Report
            [0x10, 0x01, b2, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
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
            // Aux Report - the button state is the byte at index 23, whatever the report length
            [0x10, 0x06, ..] if data.len() >= 24 => {
                let b23 = data.get(23).copied().unwrap_or(0);
                let mut buttons: u8 = 0;
                if b23 == 1 {
                    buttons |= 1 << 0;
                }
                if b23 == 2 {
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    fn tablet_report(b2: u8, x: u16, y: u16, pressure: u16) -> [u8; 9] {
        let (x, y, p) = (x.to_le_bytes(), y.to_le_bytes(), pressure.to_le_bytes());
        [0x10, 0x01, b2, x[0], x[1], y[0], y[1], p[0], p[1]]
    }

    fn aux_report(len: usize, at_23: u8) -> Vec<u8> {
        let mut data = vec![0u8; len];
        data[0] = 0x10;
        data[1] = 0x06;
        data[23] = at_23;
        data
    }

    #[test]
    fn tablet_report_decodes_position_pressure_button_and_eraser() {
        let parsed = BambooPadParser
            .parse(&tablet_report(0x02 | 0x08, 0x0A0B, 0x0C0D, 300))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!((parsed.x, parsed.y, parsed.pressure), (0x0A0B, 0x0C0D, 300));
        assert_eq!(parsed.buttons, 0b1);
        assert!(parsed.eraser);
    }

    #[test]
    fn no_pressure_is_hover() {
        let parsed = BambooPadParser.parse(&tablet_report(0, 1, 1, 0)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Hover);
        assert_eq!(parsed.buttons, 0);
        assert!(!parsed.eraser);
    }

    #[test]
    fn aux_buttons_come_from_byte_23() {
        let parsed = BambooPadParser.parse(&aux_report(24, 1)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0b01);
        assert_eq!(
            BambooPadParser.parse(&aux_report(24, 2)).unwrap().buttons,
            0b10
        );
        assert_eq!(
            BambooPadParser.parse(&aux_report(24, 0)).unwrap().buttons,
            0
        );
    }

    #[test]
    fn aux_buttons_are_read_at_index_23_in_longer_reports() {
        // The state is at index 23 whatever the report length, not in the last byte.
        let mut data = aux_report(32, 2);
        data[31] = 1;
        assert_eq!(BambooPadParser.parse(&data).unwrap().buttons, 0b10);
    }

    #[test]
    fn rejects_unknown_and_truncated_reports() {
        let mut short = vec![0u8; 23];
        short[0] = 0x10;
        short[1] = 0x06;
        assert!(BambooPadParser.parse(&short).is_none());
        assert!(BambooPadParser.parse(&[0x10, 0x01, 0, 0]).is_none());
        assert!(
            BambooPadParser
                .parse(&[0x10, 0x09, 0, 0, 0, 0, 0, 0, 0])
                .is_none()
        );
        assert!(BambooPadParser.parse(&[]).is_none());
    }
}
