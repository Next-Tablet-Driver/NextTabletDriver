use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

fn parse_uclogic_aux(data: &[u8]) -> Option<TabletData> {
    match data {
        [_, _, _, _, b4, ..] => {
            let mut tablet_data = TabletData {
                status: crate::drivers::TabletStatus::Aux,
                buttons: *b4,
                is_connected: true,
                ..Default::default()
            };
            tablet_data.set_raw(data);
            Some(tablet_data)
        }
        _ => None,
    }
}

fn parse_uclogic_tablet(data: &[u8], has_tilt: bool) -> Option<TabletData> {
    match data {
        [_, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, rest @ ..] => {
            let x = u16::from_le_bytes([*x_lo, *x_hi]);
            let y = u16::from_le_bytes([*y_lo, *y_hi]);
            let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

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
            let eraser = (*b1 & 0x04) != 0;

            let (tilt_x, tilt_y) = if has_tilt {
                match rest {
                    [_, _, tx, ty, ..] => (tx.cast_signed(), ty.cast_signed()),
                    _ => (0, 0),
                }
            } else {
                (0, 0)
            };

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

pub struct UCLogicParser;

impl ReportParser for UCLogicParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0xC0, ..] => None,
            [_, b1, ..] if (*b1 & 0x40) != 0 => parse_uclogic_aux(data),
            _ => parse_uclogic_tablet(data, false),
        }
    }
}

pub struct UCLogicV1Parser;

impl ReportParser for UCLogicV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0xE0, ..] => parse_uclogic_aux(data),
            [_, b1, ..] if (*b1 & 0x40) != 0 => parse_uclogic_tablet(data, false),
            _ => None,
        }
    }
}

pub struct UCLogicV2Parser;

impl ReportParser for UCLogicV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0xE0, ..] => parse_uclogic_aux(data),
            [_, 0xF0, ..] => None,
            [_, _, ..] => parse_uclogic_tablet(data, true),
            _ => None,
        }
    }
}

pub struct UCLogicTiltParser;

impl ReportParser for UCLogicTiltParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, b1, ..] if (*b1 & 0x40) != 0 => parse_uclogic_aux(data),
            [_, _, ..] => parse_uclogic_tablet(data, true),
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
    fn test_uclogic_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = UCLogicParser;
        let data: [u8; 8] = [0, 0x01, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("UCLogic parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 1);
        Ok(())
    }

    #[test]
    fn test_uclogic_aux() -> Result<(), Box<dyn std::error::Error>> {
        let parser = UCLogicParser;
        let data: [u8; 8] = [0, 0x40, 0, 0, 5, 0, 0, 0];
        let report = parser
            .parse(&data)
            .ok_or("UCLogic parser failed to parse aux packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Aux);
        assert_eq!(report.buttons, 5);
        Ok(())
    }
}
