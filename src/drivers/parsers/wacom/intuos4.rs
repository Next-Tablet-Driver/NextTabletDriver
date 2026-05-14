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
                let x = ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1);
                let y = ((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1);
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
