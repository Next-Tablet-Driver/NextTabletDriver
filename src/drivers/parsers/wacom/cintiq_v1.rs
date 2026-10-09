use super::intuos_v1::IntuosV1Parser;
use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct CintiqV1Parser {
    inner_v1: IntuosV1Parser,
}

impl CintiqV1Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner_v1: IntuosV1Parser::new(),
        }
    }
}

impl Default for CintiqV1Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for CintiqV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02 | 0x10, ..] => self.inner_v1.parse(data), // reuse v1 tablet parsing
            [0x0C, _, _, _, _, b5, b6, _, _, h_dist, ..] => {
                let mut buttons: u32 = 0;
                if (*b5 & 1) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b6 & 1) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b6 & 2) != 0 {
                    buttons |= 1 << 2;
                }
                if (*b6 & 4) != 0 {
                    buttons |= 1 << 3;
                }
                if (*b6 & 8) != 0 {
                    buttons |= 1 << 4;
                }
                if (*b6 & 16) != 0 {
                    buttons |= 1 << 5;
                }
                if (*b6 & 32) != 0 {
                    buttons |= 1 << 6;
                }
                if (*b6 & 64) != 0 {
                    buttons |= 1 << 7;
                }
                if (*b6 & 128) != 0 {
                    buttons |= 1 << 8;
                }

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: buttons as u8,
                    hover_distance: *h_dist,
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
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    #[test]
    fn pen_reports_are_delegated_to_intuos_v1() {
        let data = [0x02, 0x27, 0x12, 0x34, 0x56, 0x78, 0x40, 0x80, 0x80, 0x03];
        let ours = CintiqV1Parser::new().parse(&data).unwrap();
        let v1 = IntuosV1Parser::new().parse(&data).unwrap();
        assert_eq!(ours.status, TabletStatus::Contact);
        assert_eq!((ours.x, ours.y, ours.pressure), (v1.x, v1.y, v1.pressure));
    }

    #[test]
    fn express_keys_map_to_buttons_and_expose_the_hover_distance() {
        // b5 bit 0 is button 0, b6 bits 0..7 are buttons 1..8 (only 8 fit in the output).
        let data = [0x0C, 0, 0, 0, 0, 0x01, 0x06, 0, 0, 9];
        let parsed = CintiqV1Parser::default().parse(&data).unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0b1101);
        assert_eq!(parsed.hover_distance, 9);
    }

    #[test]
    fn unknown_and_truncated_reports_are_rejected() {
        let parser = CintiqV1Parser::new();
        assert!(parser.parse(&[]).is_none());
        assert!(parser.parse(&[0x77, 0, 0]).is_none());
        assert!(parser.parse(&[0x0C, 0, 0, 0, 0, 1, 1]).is_none());
    }

    #[test]
    fn every_express_key_bit_lands_on_its_own_button() {
        // b5 bit 0 is button 0, b6 bits 0..7 are buttons 1..8; only eight buttons fit in the output.
        for bit in 0..8u32 {
            let data = [0x0C, 0, 0, 0, 0, 0, 1u8 << bit, 0, 0, 0];
            let parsed = CintiqV1Parser::new().parse(&data).unwrap();
            let expected = u8::try_from((1u32 << (bit + 1)) & 0xFF).unwrap();
            assert_eq!(parsed.buttons, expected, "b6 bit {bit}");
        }
        let data = [0x0C, 0, 0, 0, 0, 0x01, 0, 0, 0, 0];
        assert_eq!(CintiqV1Parser::new().parse(&data).unwrap().buttons, 0b1);
    }
}
