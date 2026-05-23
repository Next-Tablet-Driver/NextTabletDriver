use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct XenxParser;

impl ReportParser for XenxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
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
                    eraser,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            // Aux Report
            [0x02, _, aux_data @ ..] if aux_data.len() >= 8 => {
                let mut buttons: u8 = 0;
                for (i, &val) in aux_data.iter().take(8).enumerate() {
                    if val != 0 {
                        buttons |= 1 << i;
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
    fn test_xenx_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = XenxParser;
        let data: [u8; 8] = [0x01, 0x06, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("Xenx parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 3);
        Ok(())
    }
}
