use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct XenxParser;

impl ReportParser for XenxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            // Tablet Report
            [0x01, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                if *b1 == 0 {
                    return None; // Out of range
                }

                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                let eraser = (*b1 & 0x40) != 0;

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x,
                    y,
                    pressure,
                    buttons,
                    eraser,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            // Aux Report
            [0x02, _, aux_data @ ..] if aux_data.len() >= 8 => {
                let mut buttons: u8 = 0;
                for (i, &val) in aux_data.iter().take(8).enumerate() {
                    if val != 0 {
                        buttons |= 1 << i;
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
    fn test_xenx_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = XenxParser;
        let data: [u8; 8] = [0x01, 0x06, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("Xenx parser failed to parse tablet packet")?;
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 3);
        Ok(())
    }
}
