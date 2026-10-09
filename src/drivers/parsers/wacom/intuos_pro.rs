use super::intuos_v1::IntuosV1Parser;
use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos Pro

pub struct IntuosProParser {
    inner_v1: IntuosV1Parser,
}

impl IntuosProParser {
    #[must_use]
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
    fn parse_internal(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02 | 0x10, ..] => self.inner_v1.parse_internal(data),
            [0x03, ..] => Self::parse_aux(data),
            _ => None,
        }
    }

    fn parse_aux(data: &[u8]) -> Option<TabletData> {
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
}

impl ReportParser for IntuosProParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        self.parse_internal(data)
    }
}

pub struct WacomDriverIntuosProParser {
    inner: IntuosProParser,
}

impl WacomDriverIntuosProParser {
    #[must_use]
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
        match data {
            [_, rest @ ..] => self.inner.parse_internal(rest),
            _ => None,
        }
    }
}
