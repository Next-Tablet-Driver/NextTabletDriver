use super::intuos_v1::IntuosV1Parser;
use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos Pro

pub struct IntuosProParser {
    inner_v1: IntuosV1Parser,
}

impl IntuosProParser {
    pub const fn new() -> Self {
        Self {
            inner_v1: IntuosV1Parser::new(),
        }
    }
}

impl Default for IntuosProParser {
    fn default() -> Self {
        Self::new()
    }
}

impl IntuosProParser {
    fn parse_internal(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [0x02, ..] | [0x10, ..] => self.inner_v1.parse_internal(data, raw),
            [0x03, ..] => self.parse_aux(data, raw),
            _ => None,
        }
    }

    fn parse_aux(&self, data: &[u8], raw: String) -> Option<TabletData> {
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
}

impl ReportParser for IntuosProParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.parse_internal(data, raw)
    }
}

pub struct WacomDriverIntuosProParser {
    inner: IntuosProParser,
}

impl WacomDriverIntuosProParser {
    pub const fn new() -> Self {
        Self {
            inner: IntuosProParser::new(),
        }
    }
}

impl Default for WacomDriverIntuosProParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuosProParser {
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
