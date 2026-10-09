use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos V3

pub struct IntuosV3Parser;

impl IntuosV3Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for IntuosV3Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl IntuosV3Parser {
    fn parse_internal(data: &[u8]) -> Option<TabletData> {
        match data {
            [0x11, ..] => Self::parse_aux(data),
            [0x1E, ..] => Self::parse_extended(data),
            [0x1F, 0x01, ..] => Self::parse_tablet(data),
            _ => None,
        }
    }

    fn parse_tablet(data: &[u8]) -> Option<TabletData> {
        match data {
            [
                _,
                _,
                b2,
                x_lo,
                x_hi,
                y_lo,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                _,
                t_y,
                _,
                h_dist,
                ..,
            ] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let tilt_x = if (*t_x & 0x80) != 0 {
                    (i16::from(*t_x) - 0xFF) as i8
                } else {
                    t_x.cast_signed()
                };
                let tilt_y = if (*t_y & 0x80) != 0 {
                    (i16::from(*t_y) - 0xFF) as i8
                } else {
                    t_y.cast_signed()
                };

                let mut buttons: u8 = 0;
                if (*b2 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b2 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                let eraser = (*b2 & 0x20) != 0;

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
                    hover_distance: *h_dist,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            _ => None,
        }
    }

    fn parse_extended(data: &[u8]) -> Option<TabletData> {
        match data {
            [
                _,
                _,
                b2,
                x_lo,
                x_hi,
                x_ext,
                y_lo,
                y_hi,
                y_ext,
                p_lo,
                p_hi,
                t_x_lo,
                t_x_hi,
                t_y_lo,
                t_y_hi,
                _,
                _,
                _,
                _,
                h_dist,
                ..,
            ] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi])) | (u32::from(*x_ext) << 16);
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi])) | (u32::from(*y_ext) << 16);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);
                let tilt_x = (i16::from_le_bytes([*t_x_lo, *t_x_hi])) as i8;
                let tilt_y = (i16::from_le_bytes([*t_y_lo, *t_y_hi])) as i8;

                let mut buttons: u8 = 0;
                if (*b2 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b2 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b2 & 0x08) != 0 {
                    buttons |= 1 << 2;
                }
                let eraser = (*b2 & 0x20) != 0;

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
                    hover_distance: *h_dist,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            _ => None,
        }
    }

    fn parse_aux(data: &[u8]) -> Option<TabletData> {
        match data {
            [_, b1, _, b2, ..] => {
                let mut buttons: u16 = 0;
                if (*b1 & 1) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 2) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b1 & 4) != 0 {
                    buttons |= 1 << 2;
                }
                if (*b1 & 8) != 0 {
                    buttons |= 1 << 3;
                }
                if (*b2 & 1) != 0 {
                    buttons |= 1 << 4;
                }
                if (*b1 & 16) != 0 {
                    buttons |= 1 << 5;
                }
                if (*b1 & 32) != 0 {
                    buttons |= 1 << 6;
                }
                if (*b1 & 64) != 0 {
                    buttons |= 1 << 7;
                }

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: buttons as u8,
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

impl ReportParser for IntuosV3Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        Self::parse_internal(data)
    }
}

pub struct WacomDriverIntuosV3Parser;

impl WacomDriverIntuosV3Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn parse_internal(data: &[u8]) -> Option<TabletData> {
        IntuosV3Parser::parse_internal(data)
    }
}

impl Default for WacomDriverIntuosV3Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuosV3Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        // Skip the first byte safely
        match data {
            [_, rest @ ..] => Self::parse_internal(rest),
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
    fn test_intuos_v3_tablet_report() {
        let parser = IntuosV3Parser::new();
        // Report ID 1F, status 01, Pen buttons, X, Y, Pressure, Tilt
        let mut data = [0u8; 15];
        data[0] = 0x1F;
        data[1] = 0x01;
        data[2] = 0x02; // Pen button 1
        data[3] = 0x01;
        data[4] = 0x00; // X = 1
        data[7] = 0xAA;
        data[8] = 0x00; // Pressure = 170

        let result = parser.parse(&data).expect("Should parse");
        assert_eq!(result.x, 1);
        assert_eq!(result.pressure, 170);
        assert_eq!(result.buttons, 1 << 0);
    }

    #[test]
    fn test_intuos_v3_extended_report() {
        let parser = IntuosV3Parser::new();
        // Report ID 1E (extended format), status 01, b2=0x02, x_lo=0x10, x_hi=0x20, x_ext=0x01
        let mut data = [0u8; 25];
        data[0] = 0x1E;
        data[1] = 0x01;
        data[2] = 0x02; // Pen button 1
        data[3] = 0x10; // x_lo
        data[4] = 0x20; // x_hi (0x2010 = 8208)
        data[5] = 0x01; // x_ext (0x010000 = 65536) -> total x = 73744
        data[6] = 0x05; // y_lo
        data[7] = 0x00; // y_hi
        data[8] = 0x02; // y_ext (0x020000 = 131072) -> total y = 131077
        data[9] = 0x50; // p_lo
        data[10] = 0x01; // p_hi = 336

        let result = parser.parse(&data).expect("Should parse extended report");
        assert_eq!(result.x, 73744);
        assert_eq!(result.y, 131_077);
        assert_eq!(result.pressure, 336);
        assert_eq!(result.buttons, 1 << 0);
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;
        use crate::drivers::TabletStatus;

        fn tablet(b2: u8, t_x: u8, t_y: u8) -> [u8; 14] {
            // [id, 0x01, b2, x(2), y(2), pressure(2), t_x, _, t_y, _, hover]
            [
                0x1F, 0x01, b2, 0x34, 0x12, 0x78, 0x56, 0x00, 0x01, t_x, 0, t_y, 0, 4,
            ]
        }

        #[test]
        fn tablet_report_decodes_every_field() {
            let parsed = IntuosV3Parser::new()
                .parse(&tablet(0x02 | 0x04 | 0x20, 0x05, 0x02))
                .unwrap();
            assert_eq!(parsed.status, TabletStatus::Contact);
            assert_eq!(
                (parsed.x, parsed.y, parsed.pressure),
                (0x1234, 0x5678, 0x0100)
            );
            assert_eq!((parsed.tilt_x, parsed.tilt_y), (5, 2));
            assert_eq!(parsed.buttons, 0b11);
            assert!(parsed.eraser);
            assert_eq!(parsed.hover_distance, 4);
        }

        #[test]
        fn negative_tilt_is_stored_as_an_offset_from_0xff() {
            let parsed = IntuosV3Parser::new()
                .parse(&tablet(0x00, 0xF0, 0x80))
                .unwrap();
            assert_eq!(parsed.tilt_x, -15);
            assert_eq!(parsed.tilt_y, -127);
        }

        #[test]
        fn tablet_reports_need_the_0x01_subtype() {
            let mut data = tablet(0, 0, 0);
            data[1] = 0x02;
            assert!(IntuosV3Parser::new().parse(&data).is_none());
            assert!(IntuosV3Parser::new().parse(&data[..13]).is_none());
        }

        #[test]
        fn extended_report_uses_three_byte_coordinates_and_three_buttons() {
            let mut data = [0u8; 20];
            data[0] = 0x1E;
            data[2] = 0x02 | 0x08 | 0x20;
            data[3..6].copy_from_slice(&[0x34, 0x12, 0x01]); // x
            data[6..9].copy_from_slice(&[0x78, 0x56, 0x02]); // y
            data[9..11].copy_from_slice(&[0x00, 0x03]); // pressure
            data[11..13].copy_from_slice(&[0xFE, 0xFF]); // tilt x = -2
            data[13..15].copy_from_slice(&[0x07, 0x00]); // tilt y = 7
            data[19] = 9;
            let parsed = IntuosV3Parser::new().parse(&data).unwrap();
            assert_eq!((parsed.x, parsed.y), (0x0001_1234, 0x0002_5678));
            assert_eq!(parsed.pressure, 0x0300);
            assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 7));
            assert_eq!(parsed.buttons, 0b101);
            assert!(parsed.eraser);
            assert_eq!(parsed.hover_distance, 9);
            assert!(IntuosV3Parser::new().parse(&data[..19]).is_none());
        }

        #[test]
        fn aux_report_merges_two_button_bytes() {
            // b1 bits 0..3 and 4..6 map to buttons 0..3 and 5..7, b2 bit 0 to button 4.
            let parsed = IntuosV3Parser::new()
                .parse(&[0x11, 0x15, 0x00, 0x01])
                .unwrap();
            assert_eq!(parsed.status, TabletStatus::Aux);
            assert_eq!(parsed.buttons, 0b0011_0101);
            let none = IntuosV3Parser::new()
                .parse(&[0x11, 0x00, 0x00, 0x00])
                .unwrap();
            assert_eq!(none.buttons, 0);
            assert!(IntuosV3Parser::new().parse(&[0x11, 0x15]).is_none());
        }

        #[test]
        fn unknown_ids_are_rejected_and_the_driver_variant_skips_a_byte() {
            assert!(IntuosV3Parser::new().parse(&[0x55, 1, 2, 3]).is_none());
            assert!(IntuosV3Parser::new().parse(&[]).is_none());
            let parser = WacomDriverIntuosV3Parser::new();
            let parsed = parser.parse(&[0xFF, 0x11, 0x01, 0x00, 0x00]).unwrap();
            assert_eq!((parsed.status, parsed.buttons), (TabletStatus::Aux, 1));
            assert!(parser.parse(&[]).is_none());
        }

        #[test]
        #[allow(clippy::default_constructed_unit_structs)]
        fn the_default_parsers_behave_like_the_new_ones() {
            let plain = IntuosV3Parser::default();
            assert!(plain.parse(&[0x11, 0x01, 0x00, 0x00]).is_some());
            let driver = WacomDriverIntuosV3Parser::default();
            assert!(driver.parse(&[0xFF, 0x11, 0x01, 0x00, 0x00]).is_some());
        }

        #[test]
        fn a_tablet_report_without_pressure_hovers() {
            let mut data = tablet(0x00, 0, 0);
            data[7] = 0;
            data[8] = 0;
            let parsed = IntuosV3Parser::new().parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.pressure, 0);
        }

        #[test]
        fn a_tablet_report_missing_its_last_byte_is_rejected() {
            assert!(
                IntuosV3Parser::new()
                    .parse(&tablet(0x02, 0, 0)[..13])
                    .is_none()
            );
        }

        #[test]
        fn the_extended_report_maps_the_second_button_and_hovers_without_pressure() {
            let mut data = [0u8; 20];
            data[0] = 0x1E;
            data[2] = 0x04;
            let parsed = IntuosV3Parser::new().parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.buttons, 0b10);
        }

        #[test]
        fn the_aux_keys_cover_every_button_bit_of_the_first_byte() {
            let key = |b1: u8| {
                IntuosV3Parser::new()
                    .parse(&[0x11, b1, 0x00, 0x00])
                    .unwrap()
                    .buttons
            };
            assert_eq!(key(0x01), 0b0000_0001);
            assert_eq!(key(0x02), 0b0000_0010);
            assert_eq!(key(0x04), 0b0000_0100);
            assert_eq!(key(0x08), 0b0000_1000);
            assert_eq!(key(0x10), 0b0010_0000);
            assert_eq!(key(0x20), 0b0100_0000);
            assert_eq!(key(0x40), 0b1000_0000);
        }
    }
}
