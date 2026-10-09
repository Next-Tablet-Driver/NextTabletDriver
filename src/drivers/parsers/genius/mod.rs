use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct GeniusParserV1;

impl ReportParser for GeniusParserV1 {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        if data.is_empty() {
            return None;
        }

        match data {
            [0x10, b1, b2, b3, b4, b5, b6, b7, ..] => {
                // Tablet Report
                let x = u32::from(u16::from_le_bytes([*b1, *b2]));
                let y = u32::from(u16::from_le_bytes([*b3, *b4]));
                let pressure = if (*b5 & 0x04) != 0 {
                    u16::from_le_bytes([*b6, *b7])
                } else {
                    0
                };

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
                    buttons,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            [0x11, b1, _, x_lo, x_hi, y_lo, y_hi, ..] => {
                // Mouse Report
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));

                let mut buttons: u8 = 0;
                if (*b1 & 0x01) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 2;
                }

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Mouse,
                    x,
                    y,
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

pub struct GeniusParserV2;

impl ReportParser for GeniusParserV2 {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        if data.is_empty() {
            return None;
        }

        match data {
            [0x02, b1, b2, b3, b4, b5, b6, b7, ..] => {
                // Tablet Report
                let x = u32::from(u16::from_le_bytes([*b1, *b2]));
                let y = u32::from(u16::from_le_bytes([*b3, *b4]));
                let pressure = if (*b5 & 0x04) != 0 {
                    u16::from_le_bytes([*b6, *b7])
                } else {
                    0
                };

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
                    buttons,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            [0x05, _, _, aux_byte, ..] => {
                // Aux Report
                let mut buttons: u8 = 0;

                if *aux_byte > 0 {
                    let active_index = (*aux_byte - 1) / 2;
                    if active_index < 8 {
                        buttons |= 1 << active_index;
                    }
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn test_genius_v1_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = GeniusParserV1;
        // data[0]=0x10. x=258, y=772, pressure=1
        // data[5] = 0x0C (0x04 pressure valid | 0x08 btn0)
        let data: [u8; 8] = [0x10, 0x02, 0x01, 0x04, 0x03, 0x0C, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("Genius V1 parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 1);
        Ok(())
    }

    #[test]
    fn test_genius_v2_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = GeniusParserV2;
        let data: [u8; 8] = [0x02, 0x02, 0x01, 0x04, 0x03, 0x0C, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("Genius V2 parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        Ok(())
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;
        use crate::drivers::TabletStatus;

        #[test]
        fn v1_tablet_report_decodes_position_pressure_and_buttons() {
            let data = [0x10, 0x34, 0x12, 0x78, 0x56, 0x04 | 0x08 | 0x10, 0x00, 0x01];
            let parsed = GeniusParserV1.parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Contact);
            assert_eq!(
                (parsed.x, parsed.y, parsed.pressure),
                (0x1234, 0x5678, 0x0100)
            );
            assert_eq!(parsed.buttons, 0b11);
        }

        #[test]
        fn v1_pressure_needs_the_tip_flag() {
            let parsed = GeniusParserV1
                .parse(&[0x10, 1, 0, 1, 0, 0x00, 0xFF, 0xFF])
                .unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.pressure, 0);
        }

        #[test]
        fn v1_mouse_report_has_three_buttons() {
            let parsed = GeniusParserV1
                .parse(&[0x11, 0x07, 0x00, 0x10, 0x00, 0x20, 0x00])
                .unwrap();
            assert_eq!(parsed.status, TabletStatus::Mouse);
            assert_eq!((parsed.x, parsed.y), (16, 32));
            assert_eq!(parsed.buttons, 0b111);
        }

        #[test]
        fn v2_tablet_report_uses_the_same_layout_with_id_2() {
            let data = [0x02, 0x34, 0x12, 0x78, 0x56, 0x04 | 0x10, 0x05, 0x00];
            let parsed = GeniusParserV2.parse(&data).unwrap();
            assert_eq!(parsed.status, TabletStatus::Contact);
            assert_eq!((parsed.x, parsed.y, parsed.pressure), (0x1234, 0x5678, 5));
            assert_eq!(parsed.buttons, 0b10);
        }

        #[test]
        fn v2_aux_report_selects_one_of_eight_keys() {
            let key = |byte| GeniusParserV2.parse(&[0x05, 0x00, 0x00, byte]).unwrap();
            assert_eq!(key(0).status, TabletStatus::Aux);
            assert_eq!(key(0).buttons, 0);
            assert_eq!(key(1).buttons, 0b0000_0001);
            assert_eq!(key(3).buttons, 0b0000_0010);
            assert_eq!(key(15).buttons, 0b1000_0000);
            // Beyond the eighth key nothing is reported.
            assert_eq!(key(17).buttons, 0);
        }

        #[test]
        fn empty_unknown_and_truncated_reports_are_rejected() {
            for parser in [&GeniusParserV1 as &dyn ReportParser, &GeniusParserV2] {
                assert!(parser.parse(&[]).is_none());
                assert!(parser.parse(&[0x99, 1, 2, 3, 4, 5, 6, 7]).is_none());
                assert!(parser.parse(&[0x10, 1, 2]).is_none());
            }
        }

        #[test]
        fn v2_pressure_needs_the_tip_flag() {
            let parsed = GeniusParserV2
                .parse(&[0x02, 1, 0, 1, 0, 0x00, 0xFF, 0xFF])
                .unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.pressure, 0);
        }
    }
}
