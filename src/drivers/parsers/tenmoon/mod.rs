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
                let x = (u16::from(*b1) << 8) | u16::from(*b2);
                let y = (u16::from(*b3) << 8) | u16::from(*b4);

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
}
