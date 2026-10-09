use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct LifetecParser;

impl ReportParser for LifetecParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, x_lo, x_hi, y_lo, y_hi, b5, p_lo, p_hi, ..] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*b5 & 0x08) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b5 & 0x10) != 0 {
                    buttons |= 1 << 1;
                }

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
                    tilt_x: 0,
                    tilt_y: 0,
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn test_lifetec_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = LifetecParser;
        let data: [u8; 8] = [0x02, 0x02, 0x01, 0x04, 0x03, 0x08, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("Lifetec parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 1);
        Ok(())
    }

    #[test]
    fn test_lifetec_invalid() {
        let parser = LifetecParser;
        let data: [u8; 8] = [0x03, 0x02, 0x01, 0x04, 0x03, 0x08, 0x01, 0x00];
        assert!(parser.parse(&data).is_none());
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;
        use crate::drivers::TabletStatus;

        fn report(b5: u8, pressure: u16) -> [u8; 8] {
            let p = pressure.to_le_bytes();
            [0x02, 0x34, 0x12, 0x78, 0x56, b5, p[0], p[1]]
        }

        #[test]
        fn both_buttons_and_hover_without_pressure() {
            let parsed = LifetecParser.parse(&report(0x18, 0)).unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.buttons, 0b11);
            assert_eq!((parsed.x, parsed.y), (0x1234, 0x5678));
        }

        #[test]
        fn each_button_bit_maps_to_its_own_button() {
            assert_eq!(LifetecParser.parse(&report(0x08, 1)).unwrap().buttons, 0b01);
            assert_eq!(LifetecParser.parse(&report(0x10, 1)).unwrap().buttons, 0b10);
        }

        #[test]
        fn other_ids_and_short_reports_are_rejected() {
            let mut data = report(0x00, 1);
            data[0] = 0x03;
            assert!(LifetecParser.parse(&data).is_none());
            assert!(LifetecParser.parse(&report(0x00, 1)[..7]).is_none());
        }
    }
}
