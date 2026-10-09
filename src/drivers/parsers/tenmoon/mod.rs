use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct TenMoonParser;

impl ReportParser for TenMoonParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, _, _, _, _, _, _, _, _, _, _, b11, b12, ..] if *b11 != 0xFF => {
                // Aux Report
                let mut buttons: u8 = 0;
                if *b12 == 0x31 {
                    buttons |= 1 << 0;
                }
                if *b12 == 0x33 && (*b11 & 0x80) == 0 {
                    buttons |= 1 << 1;
                }
                if *b12 == 0x33 && (*b11 & 0x40) == 0 {
                    buttons |= 1 << 2;
                }
                if *b12 == 0x33 && (*b11 & 0x20) == 0 {
                    buttons |= 1 << 3;
                }
                if *b12 == 0x33 && (*b11 & 0x10) == 0 {
                    buttons |= 1 << 4;
                }
                if *b12 == 0x33 && (*b11 & 0x08) == 0 {
                    buttons |= 1 << 5;
                }
                if *b12 == 0x23 {
                    buttons |= 1 << 6;
                }
                if *b12 == 0x32 {
                    buttons |= 1 << 7;
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
            [_, b1, b2, b3, b4, b5, b6, _, _, b9, ..] => {
                // Tablet Report
                let x = u32::from((u16::from(*b1) << 8) | u16::from(*b2));
                let y = u32::from((u16::from(*b3) << 8) | u16::from(*b4));

                let btn_pressed = (*b9 & 6) != 0;
                let pre_pressure = (u16::from(*b5) << 8) | u16::from(*b6);
                let pressure_offset = if btn_pressed { 50 } else { 0 };

                let pressure = if pre_pressure >= pressure_offset {
                    let adjusted = pre_pressure - pressure_offset;
                    0x0672_u16.saturating_sub(adjusted)
                } else {
                    0x0672
                };

                let mut buttons: u8 = 0;
                if (*b9 & 0x04) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b9 & 6) == 6 {
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
    fn test_tenmoon_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = TenMoonParser;
        let data: [u8; 13] = [0, 1, 2, 3, 4, 0x05, 0x06, 0, 0, 4, 0, 0xFF, 0];
        let report = parser
            .parse(&data)
            .ok_or("TenMoon parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 414); // 1650 - (1286 - 50)
        assert_eq!(report.buttons, 1);
        Ok(())
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;
        use crate::drivers::TabletStatus;

        /// A report of 10 bytes always takes the tablet branch.
        fn pen(pre_pressure: u16, b9: u8) -> [u8; 10] {
            let p = pre_pressure.to_be_bytes();
            [0x00, 0x12, 0x34, 0x56, 0x78, p[0], p[1], 0, 0, b9]
        }

        /// A report whose byte 11 is not 0xFF is an aux report.
        fn aux(b11: u8, b12: u8) -> [u8; 13] {
            let mut data = [0u8; 13];
            data[11] = b11;
            data[12] = b12;
            data
        }

        #[test]
        fn pen_report_inverts_the_raw_pressure() {
            let parsed = TenMoonParser.parse(&pen(100, 0x00)).unwrap();
            assert_eq!(parsed.status, TabletStatus::Contact);
            assert_eq!((parsed.x, parsed.y), (0x1234, 0x5678));
            assert_eq!(parsed.pressure, 0x0672 - 100);
            assert_eq!(parsed.buttons, 0);
        }

        #[test]
        fn pressing_a_barrel_button_offsets_the_pressure_by_50() {
            let parsed = TenMoonParser.parse(&pen(100, 0x06)).unwrap();
            assert_eq!(parsed.pressure, 0x0672 - 50);
            assert_eq!(parsed.buttons, 0b11);
            let single = TenMoonParser.parse(&pen(100, 0x04)).unwrap();
            assert_eq!(single.buttons, 0b01);
        }

        #[test]
        fn raw_pressure_below_the_offset_reads_as_the_maximum() {
            let parsed = TenMoonParser.parse(&pen(10, 0x06)).unwrap();
            assert_eq!(parsed.pressure, 0x0672);
        }

        #[test]
        fn raw_pressure_beyond_the_range_is_hover() {
            let parsed = TenMoonParser.parse(&pen(0x0800, 0x00)).unwrap();
            assert_eq!(parsed.pressure, 0);
            assert_eq!(parsed.status, TabletStatus::Hover);
        }

        #[test]
        fn aux_keys_are_decoded_from_bytes_11_and_12() {
            assert_eq!(
                TenMoonParser.parse(&aux(0x01, 0x31)).unwrap().buttons,
                0b0000_0001
            );
            assert_eq!(
                TenMoonParser.parse(&aux(0x01, 0x23)).unwrap().buttons,
                0b0100_0000
            );
            assert_eq!(
                TenMoonParser.parse(&aux(0x01, 0x32)).unwrap().buttons,
                0b1000_0000
            );
            let all_pressed = TenMoonParser.parse(&aux(0x00, 0x33)).unwrap();
            assert_eq!(all_pressed.status, TabletStatus::Aux);
            assert_eq!(all_pressed.buttons, 0b0011_1110);
            // A cleared bit means the key is down: with every bit set none is.
            assert_eq!(TenMoonParser.parse(&aux(0xF8, 0x33)).unwrap().buttons, 0);
            assert_eq!(
                TenMoonParser.parse(&aux(0xBF, 0x33)).unwrap().buttons,
                0b0000_0100
            );
        }

        #[test]
        fn a_0xff_marker_in_byte_11_keeps_a_long_report_on_the_pen_branch() {
            let mut data = aux(0xFF, 0x33);
            data[9] = 0x04;
            let parsed = TenMoonParser.parse(&data).unwrap();
            assert_ne!(parsed.status, TabletStatus::Aux);
            assert_eq!(parsed.buttons, 0b01);
        }

        #[test]
        fn short_reports_are_rejected() {
            assert!(TenMoonParser.parse(&[]).is_none());
            assert!(TenMoonParser.parse(&[0u8; 9]).is_none());
        }
    }
}
