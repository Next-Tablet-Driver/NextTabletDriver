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
                    x: x as u16,
                    y: y as u16,
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
mod tests {
    use super::*;

    #[test]
    fn test_inspiroy_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = InspiroyParser;
        let data: [u8; 12] = [0x08, 0x81, 0x02, 0x01, 0, 0x04, 0x03, 0, 0x01, 0x00, 0, 0];
        let report = parser
            .parse(&data)
            .ok_or("Inspiroy parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 3);
        Ok(())
    }
}
