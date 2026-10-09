use super::intuos_v1::IntuosV1Parser;
use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos 4

pub struct Intuos4Parser {
    inner_v1: IntuosV1Parser,
}

impl Intuos4Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner_v1: IntuosV1Parser::new(),
        }
    }
}

impl Default for Intuos4Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Intuos4Parser {
    fn parse_internal(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, 0xEC | 0xAC, ..] => Self::parse_mouse(data),
            [0x02 | 0x10, ..] => self.inner_v1.parse_internal(data),
            [0x0C, ..] => Self::parse_aux(data),
            _ => None,
        }
    }

    fn parse_mouse(data: &[u8]) -> Option<TabletData> {
        match data {
            [_, _, b2, b3, b4, b5, b6, _, _, b9, ..] => {
                let x = u32::from(
                    ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1),
                );
                let y =
                    u32::from(((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1));
                let mut buttons: u8 = 0;
                if (*b6 & 0x01) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b6 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b6 & 0x02) != 0 {
                    buttons |= 1 << 2;
                }
                if (*b6 & 0x08) != 0 {
                    buttons |= 1 << 3;
                }
                if (*b6 & 0x10) != 0 {
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

    fn parse_aux(data: &[u8]) -> Option<TabletData> {
        match data {
            [_, _, _, b3, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: *b3,
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

impl ReportParser for Intuos4Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        self.parse_internal(data)
    }
}

pub struct WacomDriverIntuos4Parser {
    inner: Intuos4Parser,
}

impl WacomDriverIntuos4Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: Intuos4Parser::new(),
        }
    }
}

impl Default for WacomDriverIntuos4Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuos4Parser {
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

    /// `[id, b1, x(2), y(2), buttons, _, _, flags]`: x = 0x2469, y = 0xACF0 (flags = 0b10).
    fn mouse(b1: u8, b6: u8) -> [u8; 10] {
        [0x02, b1, 0x12, 0x34, 0x56, 0x78, b6, 0, 0, 0b10]
    }

    #[test]
    fn mouse_reports_decode_position_and_five_buttons() {
        for b1 in [0xEC, 0xAC] {
            let parsed = Intuos4Parser::new()
                .parse(&mouse(b1, 0x01 | 0x04 | 0x02 | 0x08 | 0x10))
                .unwrap();
            assert_eq!(parsed.status, TabletStatus::Mouse);
            assert_eq!((parsed.x, parsed.y), (0x2469, 0xACF0));
            assert_eq!(parsed.buttons, 0b1_1111);
        }
        // Each button bit lands on its own output bit.
        assert_eq!(
            Intuos4Parser::new()
                .parse(&mouse(0xEC, 0x04))
                .unwrap()
                .buttons,
            0b10
        );
        assert_eq!(
            Intuos4Parser::new()
                .parse(&mouse(0xEC, 0x02))
                .unwrap()
                .buttons,
            0b100
        );
    }

    #[test]
    fn pen_reports_are_delegated_to_intuos_v1() {
        let ours = Intuos4Parser::new().parse(&V1_TOOL).unwrap();
        let v1 = IntuosV1Parser::new().parse(&V1_TOOL).unwrap();
        assert_eq!(ours.status, TabletStatus::Contact);
        assert_eq!((ours.x, ours.y, ours.pressure), (v1.x, v1.y, v1.pressure));
    }

    #[test]
    fn express_key_report_exposes_byte_3_as_buttons() {
        let parsed = Intuos4Parser::new().parse(&[0x0C, 0, 0, 0x2A]).unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0x2A);
    }

    #[test]
    fn driver_variant_skips_the_leading_byte() {
        let mut framed = vec![0xFF];
        framed.extend_from_slice(&mouse(0xEC, 0x01));
        let parsed = WacomDriverIntuos4Parser::new().parse(&framed).unwrap();
        assert_eq!((parsed.status, parsed.buttons), (TabletStatus::Mouse, 1));
        assert!(WacomDriverIntuos4Parser::default().parse(&[]).is_none());
    }

    #[test]
    fn unknown_and_truncated_reports_are_rejected() {
        let parser = Intuos4Parser::default();
        assert!(parser.parse(&[]).is_none());
        assert!(parser.parse(&[0x03, 0, 0, 0, 1]).is_none());
        assert!(parser.parse(&[0x02, 0xEC, 1, 2]).is_none());
        assert!(parser.parse(&[0x0C, 0, 0]).is_none());
    }
}
