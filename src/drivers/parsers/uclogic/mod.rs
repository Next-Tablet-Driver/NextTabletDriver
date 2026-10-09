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
            let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
            let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
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
#[allow(clippy::float_cmp)]
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

    use crate::drivers::TabletStatus;

    /// `[id, b1, x(2), y(2), pressure(2), _, _, tilt_x, tilt_y]`.
    fn pen(b1: u8) -> [u8; 12] {
        [
            0x08, b1, 0x34, 0x12, 0x78, 0x56, 0x00, 0x01, 0, 0, 0xFE, 0x05,
        ]
    }

    fn aux(b1: u8, keys: u8) -> [u8; 5] {
        [0x08, b1, 0, 0, keys]
    }

    #[test]
    fn the_plain_parser_decodes_pen_reports_without_tilt() {
        let parsed = UCLogicParser.parse(&pen(0x05)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!(
            (parsed.x, parsed.y, parsed.pressure),
            (0x1234, 0x5678, 0x0100)
        );
        assert_eq!(parsed.buttons, 0b101);
        assert!(parsed.eraser);
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (0, 0));
    }

    #[test]
    fn no_pressure_is_hover() {
        let mut data = pen(0x01);
        data[6] = 0;
        data[7] = 0;
        let parsed = UCLogicParser.parse(&data).unwrap();
        assert_eq!(parsed.status, TabletStatus::Hover);
        assert_eq!(parsed.buttons, 0b001);
        assert!(!parsed.eraser);
    }

    #[test]
    fn the_plain_parser_routes_aux_and_ignores_proximity_out() {
        let parsed = UCLogicParser.parse(&aux(0x40, 0x09)).unwrap();
        assert_eq!((parsed.status, parsed.buttons), (TabletStatus::Aux, 9));
        assert!(UCLogicParser.parse(&pen(0xC0)).is_none());
    }

    #[test]
    fn v1_only_accepts_flagged_pen_reports_and_aux_reports() {
        let parsed = UCLogicV1Parser.parse(&pen(0x41)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!(parsed.buttons, 0b001);
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (0, 0));
        let aux = UCLogicV1Parser.parse(&aux(0xE0, 0x03)).unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 3));
        assert!(UCLogicV1Parser.parse(&pen(0x01)).is_none());
    }

    #[test]
    fn v2_reads_the_tilt_and_treats_0xf0_as_proximity_out() {
        let parsed = UCLogicV2Parser.parse(&pen(0x01)).unwrap();
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 5));
        let aux = UCLogicV2Parser.parse(&aux(0xE0, 0x06)).unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 6));
        assert!(UCLogicV2Parser.parse(&pen(0xF0)).is_none());
    }

    #[test]
    fn the_tilt_parser_reads_tilt_and_routes_flagged_reports_to_aux() {
        let parsed = UCLogicTiltParser.parse(&pen(0x01)).unwrap();
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 5));
        let aux = UCLogicTiltParser.parse(&aux(0x40, 0x0C)).unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 12));
    }

    #[test]
    fn tilt_is_zero_when_the_report_has_no_tilt_bytes() {
        let parsed = UCLogicV2Parser.parse(&pen(0x01)[..10]).unwrap();
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (0, 0));
    }

    #[test]
    fn short_reports_are_rejected() {
        for parser in [
            &UCLogicParser as &dyn ReportParser,
            &UCLogicV1Parser,
            &UCLogicV2Parser,
            &UCLogicTiltParser,
        ] {
            assert!(parser.parse(&[]).is_none());
            assert!(parser.parse(&[0x08]).is_none());
            assert!(parser.parse(&[0x08, 0x01, 0x02, 0x03]).is_none());
        }
    }

    #[test]
    fn the_second_barrel_button_maps_to_button_one() {
        let mut data = [0x08, 0x02, 0x34, 0x12, 0x78, 0x56, 0x00, 0x01];
        let parsed = UCLogicParser.parse(&data).unwrap();
        assert_eq!(parsed.buttons, 0b10);
        assert!(!parsed.eraser);
        data[1] = 0x07;
        assert_eq!(UCLogicParser.parse(&data).unwrap().buttons, 0b111);
    }

    #[test]
    fn an_aux_report_too_short_to_hold_the_keys_is_rejected() {
        assert!(UCLogicParser.parse(&[0x08, 0x40, 0x00]).is_none());
        assert!(UCLogicV1Parser.parse(&[0x08, 0xE0, 0x00, 0x00]).is_none());
    }
}
