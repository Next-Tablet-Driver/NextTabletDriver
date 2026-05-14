use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

fn parse_uclogic_aux(data: &[u8], raw: String) -> Option<TabletData> {
    match data {
        [_, _, _, _, b4, ..] => Some(TabletData {
            status: "Aux".to_string(),
            buttons: *b4,
            raw_data: raw,
            is_connected: true,
            ..Default::default()
        }),
        _ => None,
    }
}

fn parse_uclogic_tablet(data: &[u8], raw: String, has_tilt: bool) -> Option<TabletData> {
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

            let status = if pressure > 0 { "Contact" } else { "Hover" };

            Some(TabletData {
                status: status.to_string(),
                x,
                y,
                pressure,
                tilt_x,
                tilt_y,
                buttons,
                eraser,
                raw_data: raw,
                is_connected: true,
                ..Default::default()
            })
        }
        _ => None,
    }
}

pub struct UCLogicParser;

impl ReportParser for UCLogicParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0xC0, ..] => None,
            [_, b1, ..] if (*b1 & 0x40) != 0 => parse_uclogic_aux(data, raw),
            _ => parse_uclogic_tablet(data, raw, false),
        }
    }
}

pub struct UCLogicV1Parser;

impl ReportParser for UCLogicV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0xE0, ..] => parse_uclogic_aux(data, raw),
            [_, b1, ..] if (*b1 & 0x40) != 0 => parse_uclogic_tablet(data, raw, false),
            _ => None,
        }
    }
}

pub struct UCLogicV2Parser;

impl ReportParser for UCLogicV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0xE0, ..] => parse_uclogic_aux(data, raw),
            [_, 0xF0, ..] => None,
            [_, _, ..] => parse_uclogic_tablet(data, raw, true),
            _ => None,
        }
    }
}

pub struct UCLogicTiltParser;

impl ReportParser for UCLogicTiltParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, b1, ..] if (*b1 & 0x40) != 0 => parse_uclogic_aux(data, raw),
            [_, _, ..] => parse_uclogic_tablet(data, raw, true),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uclogic_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = UCLogicParser;
        let data: [u8; 8] = [0, 0x01, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("UCLogic parser failed to parse tablet packet")?;
        assert_eq!(report.status, "Contact");
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
        assert_eq!(report.status, "Aux");
        assert_eq!(report.buttons, 5);
        Ok(())
    }
}
