use super::intuos_v1::IntuosV1Parser;
use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos 4

pub struct Intuos4Parser {
    inner_v1: IntuosV1Parser,
}

impl Intuos4Parser {
    pub fn new() -> Self {
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
    fn parse_internal(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [0x02, 0xEC, ..] | [0x02, 0xAC, ..] => self.parse_mouse(data, raw),
            [0x02, ..] | [0x10, ..] => self.inner_v1.parse_internal(data, raw),
            [0x0C, ..] => self.parse_aux(data, raw),
            _ => None,
        }
    }

    fn parse_mouse(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [_, _, b2, b3, b4, b5, b6, _, _, b9, ..] => {
                let x = (((*b2 as u16) << 8) | (*b3 as u16)) << 1 | (((*b9 >> 1) & 1) as u16);
                let y = (((*b4 as u16) << 8) | (*b5 as u16)) << 1 | ((*b9 & 1) as u16);
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

                Some(TabletData {
                    status: "Mouse".to_string(),
                    x,
                    y,
                    buttons,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }

    fn parse_aux(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [_, _, _, b3, ..] => Some(TabletData {
                status: "Aux".to_string(),
                buttons: *b3,
                raw_data: raw,
                is_connected: true,
                ..Default::default()
            }),
            _ => None,
        }
    }
}

impl ReportParser for Intuos4Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.parse_internal(data, raw)
    }
}

pub struct WacomDriverIntuos4Parser {
    inner: Intuos4Parser,
}

impl WacomDriverIntuos4Parser {
    pub fn new() -> Self {
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
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, rest @ ..] => self.inner.parse_internal(rest, raw),
            _ => None,
        }
    }
}
