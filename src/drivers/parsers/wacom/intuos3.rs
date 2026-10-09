use super::intuos_v1::IntuosV1Parser;
use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos 3

pub struct Intuos3Parser {
    inner_v1: IntuosV1Parser,
}

impl Intuos3Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner_v1: IntuosV1Parser::new(),
        }
    }
}

impl Default for Intuos3Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Intuos3Parser {
    pub(crate) fn parse_internal(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, b1, ..] if (0xF0..=0xFF).contains(b1) || (0xB0..=0xBF).contains(b1) => {
                Self::parse_mouse(data)
            }
            [0x02 | 0x10, ..] => self.inner_v1.parse_internal(data),
            [0x03, ..] => IntuosV1Parser::parse_aux(data),
            [0x0C, ..] => Self::parse_aux(data, false),
            _ => None,
        }
    }

    fn parse_mouse(data: &[u8]) -> Option<TabletData> {
        match data {
            [_, _, b2, b3, b4, b5, _, _, b8, b9, ..] => {
                let x = u32::from(
                    ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1),
                );
                let y =
                    u32::from(((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1));
                let mut buttons: u8 = 0;
                if (*b8 & 0x04) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b8 & 0x10) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b8 & 0x08) != 0 {
                    buttons |= 1 << 2;
                }
                if (*b8 & 0x20) != 0 {
                    buttons |= 1 << 3;
                }
                if (*b8 & 0x40) != 0 {
                    buttons |= 1 << 4;
                }

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Mouse,
                    x,
                    y,
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

    pub(crate) fn parse_aux(data: &[u8], extra: bool) -> Option<TabletData> {
        match data {
            [_, _, _, _, _, b5, b6, ..] => {
                let mut buttons: u16 = 0;
                if (*b5 & 1) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b5 & 2) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b5 & 4) != 0 {
                    buttons |= 1 << 2;
                }
                if (*b5 & 8) != 0 {
                    buttons |= 1 << 3;
                }
                if (*b6 & 1) != 0 {
                    buttons |= 1 << 4;
                }
                if (*b6 & 2) != 0 {
                    buttons |= 1 << 5;
                }
                if (*b6 & 4) != 0 {
                    buttons |= 1 << 6;
                }
                if (*b6 & 8) != 0 {
                    buttons |= 1 << 7;
                }

                if extra {
                    if (*b5 & 16) != 0 {
                        buttons |= 1 << 8;
                    }
                    if (*b6 & 16) != 0 {
                        buttons |= 1 << 9;
                    }
                }

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: buttons as u8,
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

impl ReportParser for Intuos3Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        self.parse_internal(data)
    }
}

pub struct Intuos3ExtraAuxParser {
    inner: Intuos3Parser,
}

impl Intuos3ExtraAuxParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: Intuos3Parser::new(),
        }
    }
}

impl Default for Intuos3ExtraAuxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for Intuos3ExtraAuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x0C, ..] => Intuos3Parser::parse_aux(data, true),
            _ => self.inner.parse_internal(data),
        }
    }
}

pub struct WacomDriverIntuos3Parser {
    inner: Intuos3Parser,
}

impl WacomDriverIntuos3Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: Intuos3Parser::new(),
        }
    }
}

impl Default for WacomDriverIntuos3Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuos3Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, rest @ ..] => self.inner.parse_internal(rest),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    const V1_TOOL: [u8; 10] = [0x02, 0x27, 0x12, 0x34, 0x56, 0x78, 0x40, 0x80, 0x80, 0x03];

    /// `[id, b1, x(2), y(2), _, _, buttons, flags]`: a mouse report, x = 0x2469, y = 0xACF0.
    fn mouse(b1: u8, b8: u8, b9: u8) -> [u8; 10] {
        [0x02, b1, 0x12, 0x34, 0x56, 0x78, 0, 0, b8, b9]
    }

    #[test]
    fn mouse_reports_decode_position_and_five_buttons() {
        let parsed = Intuos3Parser::new()
            .parse(&mouse(0xF0, 0x04 | 0x10 | 0x08 | 0x20 | 0x40, 0b10))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Mouse);
        assert_eq!((parsed.x, parsed.y), (0x2469, 0xACF0));
        assert_eq!(parsed.buttons, 0b1_1111);
    }

    #[test]
    fn both_mouse_id_ranges_are_recognised() {
        for b1 in [0xB0, 0xBF, 0xF0, 0xFF] {
            let parsed = Intuos3Parser::new().parse(&mouse(b1, 0, 0)).unwrap();
            assert_eq!(parsed.status, TabletStatus::Mouse, "b1 = {b1:#04x}");
        }
    }

    #[test]
    fn pen_reports_are_delegated_to_intuos_v1() {
        let ours = Intuos3Parser::new().parse(&V1_TOOL).unwrap();
        let v1 = IntuosV1Parser::new().parse(&V1_TOOL).unwrap();
        assert_eq!(ours.status, TabletStatus::Contact);
        assert_eq!(
            (ours.x, ours.y, ours.pressure, ours.tilt_x, ours.tilt_y),
            (v1.x, v1.y, v1.pressure, v1.tilt_x, v1.tilt_y)
        );
    }

    #[test]
    fn id_3_is_the_v1_aux_report() {
        let parsed = Intuos3Parser::new().parse(&[0x03, 0, 0, 0, 0x07]).unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 7);
    }

    #[test]
    fn express_key_report_maps_eight_buttons() {
        // b5 low nibble -> buttons 0..3, b6 low nibble -> buttons 4..7.
        let parsed = Intuos3Parser::new()
            .parse(&[0x0C, 0, 0, 0, 0, 0x05, 0x0A])
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0b1010_0101);
    }

    #[test]
    fn extra_aux_parser_handles_express_keys_and_defers_everything_else() {
        let parser = Intuos3ExtraAuxParser::new();
        let aux = parser.parse(&[0x0C, 0, 0, 0, 0, 0x01, 0x01]).unwrap();
        assert_eq!((aux.status, aux.buttons), (TabletStatus::Aux, 0b0001_0001));
        assert_eq!(
            parser.parse(&mouse(0xF0, 0, 0)).unwrap().status,
            TabletStatus::Mouse
        );
    }

    #[test]
    fn driver_variant_skips_the_leading_byte() {
        let mut framed = vec![0xFF];
        framed.extend_from_slice(&mouse(0xF0, 0x04, 0));
        let parsed = WacomDriverIntuos3Parser::new().parse(&framed).unwrap();
        assert_eq!((parsed.status, parsed.buttons), (TabletStatus::Mouse, 1));
        assert!(WacomDriverIntuos3Parser::default().parse(&[]).is_none());
    }

    #[test]
    fn unknown_and_truncated_reports_are_rejected() {
        let parser = Intuos3Parser::default();
        assert!(parser.parse(&[]).is_none());
        assert!(parser.parse(&[0x55, 1, 2, 3]).is_none());
        assert!(parser.parse(&[0x02, 0xF0, 1, 2]).is_none());
        assert!(parser.parse(&[0x0C, 0, 0, 0, 0]).is_none());
    }

    #[test]
    fn each_express_key_bit_lands_on_its_own_button() {
        for bit in 0..4u8 {
            let low = [0x0C, 0, 0, 0, 0, 1 << bit, 0];
            assert_eq!(Intuos3Parser::new().parse(&low).unwrap().buttons, 1 << bit);
            let high = [0x0C, 0, 0, 0, 0, 0, 1 << bit];
            assert_eq!(
                Intuos3Parser::new().parse(&high).unwrap().buttons,
                1 << (bit + 4)
            );
        }
    }

    #[test]
    fn the_two_extra_keys_do_not_fit_in_the_button_byte() {
        // The extra-aux layout adds buttons 8 and 9, which the 8-bit output cannot represent.
        let parser = Intuos3ExtraAuxParser::default();
        let extra_only = parser.parse(&[0x0C, 0, 0, 0, 0, 0x10, 0x10]).unwrap();
        assert_eq!(
            (extra_only.status, extra_only.buttons),
            (TabletStatus::Aux, 0)
        );
        let everything = parser.parse(&[0x0C, 0, 0, 0, 0, 0x1F, 0x1F]).unwrap();
        assert_eq!(everything.buttons, 0xFF);
        // The plain parser ignores those bits altogether.
        let plain = Intuos3Parser::new()
            .parse(&[0x0C, 0, 0, 0, 0, 0x10, 0x10])
            .unwrap();
        assert_eq!(plain.buttons, 0);
    }
}
