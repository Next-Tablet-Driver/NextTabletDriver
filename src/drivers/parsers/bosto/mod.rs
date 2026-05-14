use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BostoParser;

impl ReportParser for BostoParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] if *b1 != 0x00 => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*b1 & 0x20) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 1;
                }

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x,
                    y,
                    pressure,
                    tilt_x: 0,
                    tilt_y: 0,
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
    fn test_bosto_pen_contact() -> Result<(), Box<dyn std::error::Error>> {
        let parser = BostoParser;
        let data: [u8; 8] = [0, 0x22, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("Bosto parser failed to parse contact packet")?;
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 3); // 0x20 and 0x02 map to bits 0 and 1
        Ok(())
    }

    #[test]
    fn test_bosto_out_of_range() {
        let parser = BostoParser;
        let data: [u8; 8] = [0, 0x00, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        assert!(parser.parse(&data).is_none());
    }
}
