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
                let x = u32::from((u16::from(*x_high) << 8) | u16::from(*x_low));
                let y = u32::from((u16::from(*y_high) << 8) | u16::from(*y_low));
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    #[test]
    fn test_g640_short_packet_returns_none() {
        let parser = XpPenStarG640Parser;
        assert!(parser.parse(&[]).is_none());
        assert!(parser.parse(&[0x02, 0xA0, 0x00, 0x00]).is_none());
        assert!(
            parser
                .parse(&[0x02, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x00])
                .is_none()
        );
    }

    #[test]
    fn test_g640_hover_status_and_coordinates() {
        let parser = XpPenStarG640Parser;
        // report_id=0x02, b1=0xA0 (Hover), x=4660 (0x1234: low=0x34, high=0x12), y=9250 (0x2422: low=0x22, high=0x24), p=0
        let report = [0x02, 0xA0, 0x34, 0x12, 0x22, 0x24, 0x00, 0x00];
        let data = parser.parse(&report).expect("Valid report should parse");

        assert_eq!(data.status, TabletStatus::Hover);
        assert_eq!(data.x, 4660);
        assert_eq!(data.y, 9250);
        assert_eq!(data.pressure, 0);
        assert_eq!(data.buttons, 0);
        assert!(!data.eraser);
        assert!(data.is_connected);
    }

    #[test]
    fn test_g640_contact_pressure_and_tilt() {
        let parser = XpPenStarG640Parser;
        // report_id=0x02, b1=0xA1 (Contact), x=1000, y=2000, pressure=1312 (0x0520), tilt_x=15, tilt_y=-20 (0xEC cast_signed)
        let report = [0x02, 0xA1, 0xE8, 0x03, 0xD0, 0x07, 0x20, 0x05, 0x0F, 0xEC];
        let data = parser
            .parse(&report)
            .expect("Valid report with tilt should parse");

        assert_eq!(data.status, TabletStatus::Contact);
        assert_eq!(data.x, 1000);
        assert_eq!(data.y, 2000);
        assert_eq!(data.pressure, 1312);
        assert_eq!(data.tilt_x, 15);
        assert_eq!(data.tilt_y, -20);
        assert!(data.is_connected);
    }

    #[test]
    fn test_g640_buttons_and_eraser() {
        let parser = XpPenStarG640Parser;
        // b1: bit 1 = button 1 (0x02), bit 2 = button 2 (0x04), bit 3 = eraser (0x08)
        // b1 = 0x02 | 0x08 = 0x0A (button 1 + eraser)
        let report = [0x02, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data = parser.parse(&report).expect("Valid report should parse");

        assert_eq!(data.buttons, 1);
        assert!(data.eraser);

        // b1 = 0x04 (button 2, no eraser)
        let report_b2 = [0x02, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data_b2 = parser.parse(&report_b2).expect("Valid report should parse");
        assert_eq!(data_b2.buttons, 2);
        assert!(!data_b2.eraser);

        // b1 = 0x06 (button 1 and 2: bits 1 & 2 set)
        let report_b12 = [0x02, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data_b12 = parser
            .parse(&report_b12)
            .expect("Valid report should parse");
        assert_eq!(data_b12.buttons, 3);
    }

    #[test]
    fn test_g640_out_of_range() {
        let parser = XpPenStarG640Parser;
        // b1 = 0xC0
        let report_c0 = [0x02, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data_c0 = parser.parse(&report_c0).expect("Valid report should parse");
        assert_eq!(data_c0.status, TabletStatus::OutOfRange);
        assert!(!data_c0.is_connected);

        // b1 = 0x00
        let report_00 = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data_00 = parser.parse(&report_00).expect("Valid report should parse");
        assert_eq!(data_00.status, TabletStatus::OutOfRange);
        assert!(!data_00.is_connected);

        // b1 with MSB set: 0x80
        let report_80 = [0x02, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data_80 = parser.parse(&report_80).expect("Valid report should parse");
        assert_eq!(data_80.status, TabletStatus::OutOfRange);
        assert!(!data_80.is_connected);
    }
}
