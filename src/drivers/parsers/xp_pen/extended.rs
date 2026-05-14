use crate::drivers::TabletData;
use crate::drivers::parsers::{ReportParser, xp_pen::standard::parse as standard_parse};

fn parse_aux(data: &[u8], raw: String, offset: usize) -> TabletData {
    let buttons = data.get(offset).copied().unwrap_or(0);
    TabletData {
        status: "Aux".to_string(),
        buttons,
        raw_data: raw,
        is_connected: true,
        ..Default::default()
    }
}

fn parse_gen2(data: &[u8], raw: String) -> Option<TabletData> {
    match data {
        [
            _,
            b1,
            x_lo,
            _,
            y_lo,
            _,
            p_lo,
            p_hi,
            t_x,
            t_y,
            x_ext,
            y_ext,
            _,
            p_ext,
            ..,
        ] => {
            let x = u32::from(*x_lo) | (u32::from(*x_ext) << 16);
            let y = u32::from(*y_lo) | (u32::from(*y_ext) << 16);
            let pressure =
                (u16::from_le_bytes([*p_lo, *p_hi]) & 0xBFFF) | (u16::from(*p_ext & 0x01) << 13);

            let mut buttons: u8 = 0;
            if (*b1 & 0x02) != 0 {
                buttons |= 1 << 0;
            }
            if (*b1 & 0x04) != 0 {
                buttons |= 1 << 1;
            }
            let eraser = (*b1 & 0x08) != 0;

            let status = if pressure > 0 { "Contact" } else { "Hover" };

            Some(TabletData {
                status: status.to_string(),
                x: x.min(0xFFFF) as u16,
                y: y.min(0xFFFF) as u16,
                pressure,
                tilt_x: t_x.cast_signed(),
                tilt_y: t_y.cast_signed(),
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

fn parse_offset_pressure(data: &[u8], raw: String, has_tilt: bool) -> Option<TabletData> {
    match data {
        [_, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, rest @ ..] => {
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
            let eraser = (*b1 & 0x08) != 0;

            let (tilt_x, tilt_y) = if has_tilt {
                match rest {
                    [tx, ty, ..] => (tx.cast_signed(), ty.cast_signed()),
                    [tx, ..] => (tx.cast_signed(), 0),
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

pub struct XpPenGen2Parser;

impl ReportParser for XpPenGen2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0xF0, ..] => Some(parse_aux(data, raw, 2)),
            [_, b1, ..] if (*b1 & 0xF0) == 0xA0 => parse_gen2(data, raw),
            _ => standard_parse(data),
        }
    }
}

pub struct XpPenDeco03Parser;

impl ReportParser for XpPenDeco03Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0xF0, ..] => Some(parse_aux(data, raw, 2)),
            [_, b1, ..] if (*b1 & 0x10) != 0 => Some(parse_aux(data, raw, 2)),
            _ => standard_parse(data),
        }
    }
}

pub struct XpPenOffsetPressureParser;

impl ReportParser for XpPenOffsetPressureParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, b1, ..] if (*b1 & 0x10) != 0 => Some(parse_aux(data, raw, 2)),
            _ if data.len() >= 10 => parse_offset_pressure(data, raw, true),
            _ => parse_offset_pressure(data, raw, false),
        }
    }
}

pub struct XpPenOffsetAuxParser;

impl ReportParser for XpPenOffsetAuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, b1, ..] if (*b1 & 0x20) != 0 => Some(parse_aux(data, raw, 4)),
            _ => standard_parse(data),
        }
    }
}

pub struct XpPenParser;

impl ReportParser for XpPenParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, b1, ..] if (*b1 & 0x10) != 0 => Some(parse_aux(data, raw, 2)),
            _ => standard_parse(data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_pen_gen2() -> Result<(), Box<dyn std::error::Error>> {
        let parser = XpPenGen2Parser;
        let data: [u8; 14] = [
            0, 0xA2, 0x02, 0, 0x04, 0, 0x01, 0x00, 10, 20, 0x01, 0x03, 0, 0,
        ];
        let report = parser
            .parse(&data)
            .ok_or("XP-Pen Gen2 parser failed to parse tablet packet")?;
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 0xFFFF); // overflow u16 max clamped
        assert_eq!(report.buttons, 1);
        Ok(())
    }
}
