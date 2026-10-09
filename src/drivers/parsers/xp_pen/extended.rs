use crate::drivers::TabletData;
use crate::drivers::parsers::{ReportParser, xp_pen::standard::parse as standard_parse};

fn parse_aux(data: &[u8], offset: usize) -> TabletData {
    let buttons = data.get(offset).copied().unwrap_or(0);
    let mut tablet_data = TabletData {
        status: crate::drivers::TabletStatus::Aux,
        buttons,
        is_connected: true,
        ..Default::default()
    };
    tablet_data.set_raw(data);
    tablet_data
}

fn parse_gen2(data: &[u8]) -> Option<TabletData> {
    match data {
        [
            _,
            b1,
            x_lo,
            x_hi,
            y_lo,
            y_hi,
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
            let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi])) | (u32::from(*x_ext) << 16);
            let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi])) | (u32::from(*y_ext) << 16);
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
                tilt_x: t_x.cast_signed(),
                tilt_y: t_y.cast_signed(),
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

fn parse_offset_pressure(data: &[u8], has_tilt: bool) -> Option<TabletData> {
    match data {
        [_, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, rest @ ..] => {
            let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
            let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
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

pub struct XpPenGen2Parser;

impl ReportParser for XpPenGen2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0xF0, ..] => Some(parse_aux(data, 2)),
            [_, b1, ..] if (*b1 & 0xF0) == 0xA0 => parse_gen2(data),
            _ => standard_parse(data),
        }
    }
}

pub struct XpPenDeco03Parser;

impl ReportParser for XpPenDeco03Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0xF0, ..] => Some(parse_aux(data, 2)),
            [_, b1, ..] if (*b1 & 0x10) != 0 => Some(parse_aux(data, 2)),
            _ => standard_parse(data),
        }
    }
}

pub struct XpPenOffsetPressureParser;

impl ReportParser for XpPenOffsetPressureParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, b1, ..] if (*b1 & 0x10) != 0 => Some(parse_aux(data, 2)),
            _ if data.len() >= 10 => parse_offset_pressure(data, true),
            _ => parse_offset_pressure(data, false),
        }
    }
}

pub struct XpPenOffsetAuxParser;

impl ReportParser for XpPenOffsetAuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, b1, ..] if (*b1 & 0x20) != 0 => Some(parse_aux(data, 4)),
            _ => standard_parse(data),
        }
    }
}

pub struct XpPenParser;

impl ReportParser for XpPenParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, b1, ..] if (*b1 & 0x10) != 0 => Some(parse_aux(data, 2)),
            _ => standard_parse(data),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
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
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 65538); // 0x02 | (0x01 << 16)
        assert_eq!(report.y, 196_612); // 0x04 | (0x03 << 16)
        assert_eq!(report.buttons, 1);
        Ok(())
    }

    use crate::drivers::TabletStatus;

    /// `[id, b1, x(2), y(2), pressure(2), tilt_x, tilt_y]` padded to `len` bytes.
    fn report(b1: u8, len: usize) -> Vec<u8> {
        let mut data = vec![0u8; len];
        data[0] = 0x02;
        data[1] = b1;
        data[2..4].copy_from_slice(&[0x34, 0x12]);
        data[4..6].copy_from_slice(&[0x78, 0x56]);
        if len >= 8 {
            data[6..8].copy_from_slice(&[0x00, 0x01]);
        }
        if len >= 10 {
            data[8] = 0xFE;
            data[9] = 0x05;
        }
        data
    }

    #[test]
    fn gen2_decodes_extended_coordinates_and_pressure() {
        let mut data = report(0xA0 | 0x0A, 14);
        data[10] = 0x01; // x_ext
        data[11] = 0x02; // y_ext
        data[13] = 0x01; // pressure extension bit
        let parsed = XpPenGen2Parser.parse(&data).unwrap();
        assert_eq!((parsed.x, parsed.y), (0x0001_1234, 0x0002_5678));
        assert_eq!(parsed.pressure, 0x0100 | 0x2000);
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 5));
        assert_eq!(parsed.buttons, 0b1);
        assert!(parsed.eraser);
        assert_eq!(parsed.status, TabletStatus::Contact);
    }

    #[test]
    fn gen2_without_pressure_is_hover_and_short_gen2_reports_are_dropped() {
        let mut data = report(0xA0, 14);
        data[6] = 0;
        data[7] = 0;
        assert_eq!(
            XpPenGen2Parser.parse(&data).unwrap().status,
            TabletStatus::Hover
        );
        assert!(XpPenGen2Parser.parse(&report(0xA0, 12)).is_none());
    }

    #[test]
    fn gen2_routes_aux_and_standard_reports() {
        let aux = XpPenGen2Parser.parse(&[0x02, 0xF0, 0x07]).unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 7));
        let standard = XpPenGen2Parser.parse(&report(0x02, 10)).unwrap();
        assert_eq!(standard.status, TabletStatus::Active);
        assert_eq!(standard.buttons, 1);
    }

    #[test]
    fn deco03_treats_0x10_reports_as_aux() {
        let aux = XpPenDeco03Parser.parse(&[0x02, 0x10, 0x09]).unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 9));
        let aux = XpPenDeco03Parser.parse(&[0x02, 0xF0, 0x03]).unwrap();
        assert_eq!(aux.buttons, 3);
        let pen = XpPenDeco03Parser.parse(&report(0x02, 10)).unwrap();
        assert_eq!(pen.status, TabletStatus::Active);
    }

    #[test]
    fn offset_pressure_reads_tilt_only_from_long_reports() {
        let long = XpPenOffsetPressureParser.parse(&report(0x06, 10)).unwrap();
        assert_eq!((long.x, long.y, long.pressure), (0x1234, 0x5678, 0x0100));
        assert_eq!((long.tilt_x, long.tilt_y), (-2, 5));
        assert_eq!(long.buttons, 0b11);
        assert_eq!(long.status, TabletStatus::Contact);

        let short = XpPenOffsetPressureParser.parse(&report(0x08, 8)).unwrap();
        assert_eq!((short.tilt_x, short.tilt_y), (0, 0));
        assert!(short.eraser);

        let aux = XpPenOffsetPressureParser.parse(&report(0x10, 10)).unwrap();
        assert_eq!(aux.status, TabletStatus::Aux);
        assert!(XpPenOffsetPressureParser.parse(&[0x02, 0x00]).is_none());
    }

    #[test]
    fn offset_aux_reads_its_keys_four_bytes_in() {
        let aux = XpPenOffsetAuxParser
            .parse(&[0x02, 0x20, 0x00, 0x00, 0x07])
            .unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 7));
        // A report too short to hold the key byte reports no key rather than failing.
        assert_eq!(
            XpPenOffsetAuxParser
                .parse(&[0x02, 0x20, 0x00])
                .unwrap()
                .buttons,
            0
        );
        assert_eq!(
            XpPenOffsetAuxParser
                .parse(&report(0x02, 10))
                .unwrap()
                .status,
            TabletStatus::Active
        );
    }

    #[test]
    fn the_plain_parser_splits_aux_from_pen_reports() {
        let aux = XpPenParser.parse(&[0x02, 0x10, 0x05]).unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 5));
        assert_eq!(
            XpPenParser.parse(&report(0xA1, 10)).unwrap().status,
            TabletStatus::Contact
        );
        assert!(XpPenParser.parse(&[0x02, 0x02]).is_none());
    }

    #[test]
    fn gen2_maps_the_second_barrel_button() {
        let parsed = XpPenGen2Parser.parse(&report(0xA4, 14)).unwrap();
        assert_eq!(parsed.buttons, 0b10);
    }

    #[test]
    fn the_offset_pressure_parser_without_pressure_hovers() {
        let mut data = report(0x06, 10);
        data[6] = 0;
        data[7] = 0;
        let parsed = XpPenOffsetPressureParser.parse(&data).unwrap();
        assert_eq!(parsed.status, TabletStatus::Hover);
    }

    #[test]
    fn offset_pressure_reads_whatever_tilt_bytes_are_available() {
        // Called directly: the parser only asks for tilt on long reports, so the shorter
        // layouts are only reachable here.
        let full = report(0x06, 10);
        let one = parse_offset_pressure(&full[..9], true).unwrap();
        assert_eq!((one.tilt_x, one.tilt_y), (-2, 0));
        let none = parse_offset_pressure(&full[..8], true).unwrap();
        assert_eq!((none.tilt_x, none.tilt_y), (0, 0));
        let both = parse_offset_pressure(&full, true).unwrap();
        assert_eq!((both.tilt_x, both.tilt_y), (-2, 5));
        let ignored = parse_offset_pressure(&full, false).unwrap();
        assert_eq!((ignored.tilt_x, ignored.tilt_y), (0, 0));
    }
}
