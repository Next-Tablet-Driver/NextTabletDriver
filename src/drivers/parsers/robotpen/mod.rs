use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct RobotPenParser;

impl ReportParser for RobotPenParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0x42, _, _, _, _, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*p_hi & 0x02) != 0 {
                    buttons |= 1 << 0;
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
    fn test_robotpen_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = RobotPenParser;
        let data: [u8; 12] = [
            0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01, 0x04, 0x03, 0x01, 0x02,
        ];
        let report = parser
            .parse(&data)
            .ok_or("RobotPen parser failed to parse tablet packet")?;
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 513); // 2 | 1<<8
        assert_eq!(report.buttons, 1); // data[11] & 2 = 2 => btn0 set
        Ok(())
    }
}
