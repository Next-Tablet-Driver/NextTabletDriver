use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct InspiroyParser;

impl ReportParser for InspiroyParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            // Out of range
            [_, 0x00, ..] => None,

            // Aux Report
            [_, 0xE0 | 0xE3, _, _, b4, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: *b4,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }

            // Wheel Report
            [_, 0xF1 | 0xF0, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: 0,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }

            // Standard Tablet Report
            [
                _,
                b1,
                x_low,
                x_high,
                y_low,
                y_high,
                p_low,
                p_high,
                rest @ ..,
            ] => {
                let (b8, b9, tx, ty) = match rest {
                    [b8, b9, tx, ty, ..] => (*b8, *b9, *tx, *ty),
                    [b8, b9, tx, ..] => (*b8, *b9, *tx, 0),
                    [b8, b9, ..] => (*b8, *b9, 0, 0),
                    [b8, ..] => (*b8, 0, 0, 0),
                    _ => (0, 0, 0, 0),
                };

                let x = u32::from(*x_low) | (u32::from(*x_high) << 8) | u32::from(b8 & 1) << 16;
                let y = u32::from(*y_low) | (u32::from(*y_high) << 8) | u32::from(b9 & 1) << 16;
                let pressure = u16::from(*p_low) | (u16::from(*p_high) << 8);

                let tilt_x = tx.cast_signed().wrapping_mul(-1);
                let tilt_y = ty.cast_signed().wrapping_mul(-1);

                let buttons = (*b1 >> 1) & 0x07;
                let eraser = (*b1 & 0x10) != 0;

                let status = if pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else if (*b1 & 0x01) != 0 {
                    crate::drivers::TabletStatus::Hover
                } else {
                    crate::drivers::TabletStatus::OutOfRange
                };

                let mut tablet_data = TabletData {
                    status,
                    x,
                    y,
                    pressure,
                    tilt_x,
                    tilt_y,
                    buttons,
                    eraser,
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
    fn test_inspiroy_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = InspiroyParser;
        let data: [u8; 12] = [0x08, 0x81, 0x02, 0x01, 0, 0x04, 0x03, 0, 0x00, 0x00, 0, 0];
        let report = parser
            .parse(&data)
            .ok_or("Inspiroy parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 3);
        Ok(())
    }

    #[test]
    fn test_inspiroy_tablet_large_coordinates() -> Result<(), Box<dyn std::error::Error>> {
        let parser = InspiroyParser;
        // b8 has bit 0 set (x bit 16), b9 has bit 0 set (y bit 16)
        // x = 0x02 | (0x01 << 8) | (1 << 16) = 258 + 65536 = 65794
        // y = 0x04 | (0x02 << 8) | (1 << 16) = 516 + 65536 = 66052
        let data: [u8; 12] = [
            0x08, 0x81, 0x02, 0x01, 0x04, 0x02, 0x03, 0, 0x01, 0x01, 0, 0,
        ];
        let report = parser
            .parse(&data)
            .ok_or("Inspiroy parser failed to parse tablet packet")?;
        assert_eq!(report.x, 65794);
        assert_eq!(report.y, 66052);
        Ok(())
    }
}
