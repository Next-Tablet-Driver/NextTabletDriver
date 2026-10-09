use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos V2

pub struct IntuosV2Parser;

impl IntuosV2Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for IntuosV2Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl IntuosV2Parser {
    fn parse_internal(data: &[u8]) -> Option<TabletData> {
        match data {
            [
                0x10,
                b1,
                x_lo,
                _,
                x_hi,
                y_lo,
                _,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                t_y,
                _,
                _,
                _,
                _,
                h_dist,
                ..,
            ] => {
                let x = u32::from(*x_lo) | (u32::from(*x_hi) << 16);
                let y = u32::from(*y_lo) | (u32::from(*y_hi) << 16);
                let pressure = u16::from(*p_lo) | (u16::from(*p_hi) << 8);

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                let eraser = (*b1 & 0x10) != 0;

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
                    hover_distance: *h_dist,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            [
                0x1E,
                b1,
                _,
                x_lo,
                _,
                x_hi,
                y_lo,
                _,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                t_y,
                ..,
            ] => {
                let x = u32::from(*x_lo) | (u32::from(*x_hi) << 16);
                let y = u32::from(*y_lo) | (u32::from(*y_hi) << 16);
                let pressure = u16::from(*p_lo) | (u16::from(*p_hi) << 8);

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b1 & 0x08) != 0 {
                    buttons |= 1 << 2;
                }
                let eraser = (*b1 & 0x10) != 0;

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
                    hover_distance: *t_x,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            [0x11, b1, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: *b1,
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

impl ReportParser for IntuosV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        Self::parse_internal(data)
    }
}

pub struct WacomDriverIntuosV2Parser;

impl WacomDriverIntuosV2Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for WacomDriverIntuosV2Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuosV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, rest @ ..] => IntuosV2Parser::parse_internal(rest),
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
    fn test_intuos_v2_tablet_report() {
        let parser = IntuosV2Parser::new();
        // Report ID 0x10, X/Y/Pressure/Tilt/Buttons
        let mut data = [0u8; 17];
        data[0] = 0x10;
        data[1] = 0x02; // Pen Button 1
        data[2] = 0x34; // X low
        data[4] = 0x12; // X high
        data[8] = 0xFF; // Pressure low
        data[9] = 0x03; // Pressure high

        let result = parser.parse(&data).expect("Should parse");
        assert_eq!(result.x, 0x0012_0034);
        assert_eq!(result.buttons, 1 << 0);
        assert_eq!(result.pressure, 1023);
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;
        use crate::drivers::TabletStatus;

        #[test]
        fn tablet_report_decodes_the_split_coordinates() {
            let mut data = [0u8; 17];
            data[0] = 0x10;
            data[1] = 0x02 | 0x04 | 0x10;
            data[2] = 0x34; // x_lo
            data[4] = 0x01; // x_hi
            data[5] = 0x78; // y_lo
            data[7] = 0x02; // y_hi
            data[9] = 0x02; // pressure high byte
            data[10] = 0xFE; // tilt x
            data[11] = 0x03; // tilt y
            data[16] = 7; // hover distance
            let parsed = IntuosV2Parser::new().parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Contact);
            assert_eq!((parsed.x, parsed.y), (0x0001_0034, 0x0002_0078));
            assert_eq!(parsed.pressure, 0x200);
            assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 3));
            assert_eq!(parsed.buttons, 0b11);
            assert!(parsed.eraser);
            assert_eq!(parsed.hover_distance, 7);
        }

        #[test]
        fn no_pressure_is_hover() {
            let mut data = [0u8; 17];
            data[0] = 0x10;
            let parsed = IntuosV2Parser::new().parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert!(!parsed.eraser);
        }

        #[test]
        fn extended_report_has_three_buttons_and_reuses_tilt_x_as_hover_distance() {
            let data = [
                0x1E, 0x0E, 0x00, 0x10, 0x00, 0x01, 0x20, 0x00, 0x02, 0x05, 0x00, 0x09, 0xFF,
            ];
            let parsed = IntuosV2Parser::new().parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Contact);
            assert_eq!((parsed.x, parsed.y), (0x0001_0010, 0x0002_0020));
            assert_eq!(parsed.pressure, 5);
            assert_eq!((parsed.tilt_x, parsed.tilt_y), (9, -1));
            assert_eq!(parsed.buttons, 0b111);
            assert!(!parsed.eraser);
            assert_eq!(parsed.hover_distance, 9);
        }

        #[test]
        fn id_0x11_is_the_aux_report() {
            let parsed = IntuosV2Parser::new().parse(&[0x11, 0x0B]).unwrap();
            assert_eq!(parsed.status, TabletStatus::Aux);
            assert_eq!(parsed.buttons, 0x0B);
        }

        #[test]
        fn unknown_and_truncated_reports_are_rejected() {
            let parser = IntuosV2Parser::new();
            assert!(parser.parse(&[]).is_none());
            assert!(parser.parse(&[0x10, 1, 2, 3]).is_none());
            assert!(parser.parse(&[0x1E, 1, 2]).is_none());
            assert!(parser.parse(&[0x55, 1, 2, 3]).is_none());
            assert!(parser.parse(&[0x11]).is_none());
        }

        #[test]
        fn the_driver_variant_skips_the_leading_byte() {
            let parsed = WacomDriverIntuosV2Parser::new()
                .parse(&[0xFF, 0x11, 0x05])
                .unwrap();
            assert_eq!((parsed.status, parsed.buttons), (TabletStatus::Aux, 5));
            assert!(WacomDriverIntuosV2Parser::new().parse(&[]).is_none());
        }

        #[test]
        #[allow(clippy::default_constructed_unit_structs)]
        fn the_default_parsers_behave_like_the_new_ones() {
            let plain = IntuosV2Parser::default();
            assert!(plain.parse(&[0x11, 0x01]).is_some());
            let driver = WacomDriverIntuosV2Parser::default();
            assert!(driver.parse(&[0xFF, 0x11, 0x01]).is_some());
        }

        #[test]
        fn the_extended_report_without_pressure_hovers() {
            let mut data = [0u8; 13];
            data[0] = 0x1E;
            let parsed = IntuosV2Parser::new().parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.pressure, 0);
        }
    }
}
