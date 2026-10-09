use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct FallbackParser;

impl ReportParser for FallbackParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, status_byte, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    #[test]
    fn test_fallback_parser_short_packet_returns_none() {
        let parser = FallbackParser;
        assert!(parser.parse(&[]).is_none());
        assert!(parser.parse(&[0x02, 0xA0, 0x00]).is_none());
        assert!(
            parser
                .parse(&[0x02, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x00])
                .is_none()
        );
    }

    #[test]
    fn test_fallback_parser_contact_and_coordinates() {
        let parser = FallbackParser;
        // report_id=0x02, status=0xA1 (bit 0 set), x=1000 (0x03E8), y=2000 (0x07D0), pressure=500 (0x01F4)
        let report = [0x02, 0xA1, 0xE8, 0x03, 0xD0, 0x07, 0xF4, 0x01, 0x00, 0x00];
        let data = parser.parse(&report).expect("Failed to parse valid report");

        assert_eq!(data.status, TabletStatus::Contact);
        assert_eq!(data.x, 1000);
        assert_eq!(data.y, 2000);
        assert_eq!(data.pressure, 500);
        assert!(data.is_connected);
        assert_eq!(data.raw_len, 10);
        assert!(data.raw_hex().starts_with("02 A1 E8 03 D0 07 F4 01"));
    }

    #[test]
    fn test_fallback_parser_hover() {
        let parser = FallbackParser;
        // status=0xA0 (bit 0 not set), pressure=0
        let report = [0x02, 0xA0, 0x64, 0x00, 0xC8, 0x00, 0x00, 0x00];
        let data = parser.parse(&report).expect("Failed to parse hover report");

        assert_eq!(data.status, TabletStatus::Hover);
        assert_eq!(data.x, 100);
        assert_eq!(data.y, 200);
        assert_eq!(data.pressure, 0);
        assert!(data.is_connected);
    }

    #[test]
    fn test_fallback_parser_out_of_range() {
        let parser = FallbackParser;
        let report_c0 = [0x02, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data_c0 = parser
            .parse(&report_c0)
            .expect("Failed to parse 0xC0 report");
        assert_eq!(data_c0.status, TabletStatus::OutOfRange);
        assert!(!data_c0.is_connected);

        let report_00 = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data_00 = parser
            .parse(&report_00)
            .expect("Failed to parse 0x00 report");
        assert_eq!(data_00.status, TabletStatus::OutOfRange);
        assert!(!data_00.is_connected);
    }
}
