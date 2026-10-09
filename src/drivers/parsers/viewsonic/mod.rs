use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct ViewSonicParser;

impl ReportParser for ViewSonicParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [
                _,
                x_lo,
                x_hi,
                _,
                _,
                y_lo,
                y_hi,
                _,
                _,
                b9,
                p_lo,
                p_hi,
                t_x,
                t_y,
                ..,
            ] if (*b9 & 0b11) == 0b11 => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));

                let pressure = if (*b9 & 0x04) != 0 {
                    u16::from_le_bytes([*p_lo, *p_hi])
                } else {
                    0
                };

                let mut buttons: u8 = 0;
                if (*b9 & 0x08) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b9 & 0x10) != 0 {
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
                    tilt_x: t_x.cast_signed(),
                    tilt_y: t_y.cast_signed(),
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
    fn test_viewsonic_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = ViewSonicParser;
        let data: [u8; 14] = [
            0, 0x02, 0x01, 0, 0, 0x04, 0x03, 0, 0, 0x1F, 0x01, 0x00, 10, 20,
        ];
        let report = parser
            .parse(&data)
            .ok_or("ViewSonic parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 3);
        assert_eq!(report.tilt_x, 10);
        Ok(())
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;
        use crate::drivers::TabletStatus;

        fn report(b9: u8, pressure: u16) -> [u8; 14] {
            let p = pressure.to_le_bytes();
            [
                0, 0x34, 0x12, 0, 0, 0x78, 0x56, 0, 0, b9, p[0], p[1], 0xFE, 0x05,
            ]
        }

        #[test]
        fn pressure_needs_the_tip_flag_and_without_it_the_pen_hovers() {
            let parsed = ViewSonicParser.parse(&report(0b11, 500)).unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.pressure, 0);
            assert_eq!((parsed.x, parsed.y), (0x1234, 0x5678));
            assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 5));
        }

        #[test]
        fn a_pen_with_the_tip_flag_is_in_contact_with_both_buttons() {
            let parsed = ViewSonicParser
                .parse(&report(0b11 | 0x04 | 0x08 | 0x10, 500))
                .unwrap();
            assert_eq!(parsed.status, TabletStatus::Contact);
            assert_eq!(parsed.pressure, 500);
            assert_eq!(parsed.buttons, 0b11);
        }

        #[test]
        fn reports_without_the_in_range_bits_or_too_short_are_rejected() {
            assert!(ViewSonicParser.parse(&report(0b01, 1)).is_none());
            assert!(ViewSonicParser.parse(&report(0b11, 1)[..13]).is_none());
            assert!(ViewSonicParser.parse(&[]).is_none());
        }
    }
}
