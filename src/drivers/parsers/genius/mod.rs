use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct GeniusParserV1;

impl ReportParser for GeniusParserV1 {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        if data.is_empty() {
            return None;
        }

        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [0x10, b1, b2, b3, b4, b5, b6, b7, ..] => {
                // Tablet Report
                let x = u16::from_le_bytes([*b1, *b2]);
                let y = u16::from_le_bytes([*b3, *b4]);
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
            [0x11, b1, _, x_lo, x_hi, y_lo, y_hi, ..] => {
                // Mouse Report
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);

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

                Some(TabletData {
                    status: "Mouse".to_string(),
                    x,
                    y,
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

pub struct GeniusParserV2;

impl ReportParser for GeniusParserV2 {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        if data.is_empty() {
            return None;
        }

        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [0x02, b1, b2, b3, b4, b5, b6, b7, ..] => {
                // Tablet Report
                let x = u16::from_le_bytes([*b1, *b2]);
                let y = u16::from_le_bytes([*b3, *b4]);
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
            [0x05, _, _, aux_byte, ..] => {
                // Aux Report
                let mut buttons: u8 = 0;

                if *aux_byte > 0 {
                    let active_index = (*aux_byte - 1) / 2;
                    if active_index < 8 {
                        buttons |= 1 << active_index;
                    }
                }

                Some(TabletData {
                    status: "Aux".to_string(),
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
    fn test_genius_v1_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = GeniusParserV1;
        // data[0]=0x10. x=258, y=772, pressure=1
        // data[5] = 0x0C (0x04 pressure valid | 0x08 btn0)
        let data: [u8; 8] = [0x10, 0x02, 0x01, 0x04, 0x03, 0x0C, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("Genius V1 parser failed to parse tablet packet")?;
        assert_eq!(report.status, "Contact");
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
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        Ok(())
    }
}
