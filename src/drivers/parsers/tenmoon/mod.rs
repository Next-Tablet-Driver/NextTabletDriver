use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct TenMoonParser;

impl ReportParser for TenMoonParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

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

                Some(TabletData {
                    status: "Aux".to_string(),
                    buttons,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            [_, b1, b2, b3, b4, b5, b6, _, _, b9, ..] => {
                // Tablet Report
                let x = ((*b1 as u16) << 8) | (*b2 as u16);
                let y = ((*b3 as u16) << 8) | (*b4 as u16);

                let btn_pressed = (*b9 & 6) != 0;
                let pre_pressure = ((*b5 as u16) << 8) | (*b6 as u16);
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

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x,
                    y,
                    pressure,
                    buttons,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenmoon_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = TenMoonParser;
        let data: [u8; 13] = [0, 1, 2, 3, 4, 0x05, 0x06, 0, 0, 4, 0, 0xFF, 0];
        let report = parser
            .parse(&data)
            .ok_or("TenMoon parser failed to parse tablet packet")?;
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 414); // 1650 - (1286 - 50)
        assert_eq!(report.buttons, 1);
        Ok(())
    }
}
