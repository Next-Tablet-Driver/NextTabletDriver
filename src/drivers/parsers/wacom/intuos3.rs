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
    pub(crate) fn parse_internal(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [0x02, b1, ..] if (0xF0..=0xFF).contains(b1) || (0xB0..=0xBF).contains(b1) => {
                self.parse_mouse(data, raw)
            }
            [0x02 | 0x10, ..] => self.inner_v1.parse_internal(data, raw),
            [0x03, ..] => self.inner_v1.parse_aux(data, raw),
            [0x0C, ..] => self.parse_aux(data, raw, false),
            _ => None,
        }
    }

    fn parse_mouse(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [_, _, b2, b3, b4, b5, _, _, b8, b9, ..] => {
                let x = ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1);
                let y = ((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1);
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

    pub(crate) fn parse_aux(&self, data: &[u8], raw: String, extra: bool) -> Option<TabletData> {
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

                Some(TabletData {
                    status: "Aux".to_string(),
                    buttons: buttons as u8,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}

impl ReportParser for Intuos3Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.parse_internal(data, raw)
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
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        match data {
            [0x0C, ..] => self.inner.parse_aux(data, raw, true),
            _ => self.inner.parse_internal(data, raw),
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
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, rest @ ..] => self.inner.parse_internal(rest, raw),
            _ => None,
        }
    }
}
