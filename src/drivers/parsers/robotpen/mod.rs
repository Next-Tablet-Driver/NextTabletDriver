use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct RobotPenParser;

impl ReportParser for RobotPenParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0x42, _, _, _, _, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*p_hi & 0x02) != 0 {
                    buttons |= 1 << 0;
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
                    tilt_x: 0,
                    tilt_y: 0,
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
#[allow(clippy::float_cmp)]
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
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 513); // 2 | 1<<8
        assert_eq!(report.buttons, 1); // data[11] & 2 = 2 => btn0 set
        Ok(())
    }

    use crate::drivers::TabletStatus;

    fn report(pressure: u16) -> [u8; 12] {
        let p = pressure.to_le_bytes();
        [0, 0x42, 0, 0, 0, 0, 0x34, 0x12, 0x78, 0x56, p[0], p[1]]
    }

    #[test]
    fn a_pen_without_pressure_hovers() {
        let parsed = RobotPenParser.parse(&report(0)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Hover);
        assert_eq!((parsed.x, parsed.y), (0x1234, 0x5678));
        assert_eq!(parsed.buttons, 0);
    }

    #[test]
    fn the_button_is_flagged_in_the_pressure_high_byte() {
        let parsed = RobotPenParser.parse(&report(0x0200)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!(parsed.buttons, 1);
    }

    #[test]
    fn other_report_kinds_and_short_reports_are_rejected() {
        let mut data = report(1);
        data[1] = 0x41;
        assert!(RobotPenParser.parse(&data).is_none());
        assert!(RobotPenParser.parse(&report(1)[..11]).is_none());
        assert!(RobotPenParser.parse(&[]).is_none());
    }
}
