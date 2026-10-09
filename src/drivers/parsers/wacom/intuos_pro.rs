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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    const V1_TOOL: [u8; 10] = [0x02, 0x27, 0x12, 0x34, 0x56, 0x78, 0x40, 0x80, 0x80, 0x03];

    #[test]
    fn pen_reports_are_delegated_to_intuos_v1() {
        let ours = IntuosProParser::new().parse(&V1_TOOL).unwrap();
        let v1 = IntuosV1Parser::new().parse(&V1_TOOL).unwrap();
        assert_eq!(ours.status, TabletStatus::Contact);
        assert_eq!((ours.x, ours.y, ours.pressure), (v1.x, v1.y, v1.pressure));
    }

    #[test]
    fn aux_report_exposes_byte_4_as_buttons() {
        let parsed = IntuosProParser::default()
            .parse(&[0x03, 0, 0, 0, 0xF0])
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0xF0);
    }

    #[test]
    fn driver_variant_skips_the_leading_byte() {
        let mut framed = vec![0xFF];
        framed.extend_from_slice(&[0x03, 0, 0, 0, 0x05]);
        let parsed = WacomDriverIntuosProParser::new().parse(&framed).unwrap();
        assert_eq!((parsed.status, parsed.buttons), (TabletStatus::Aux, 5));
        assert!(WacomDriverIntuosProParser::default().parse(&[]).is_none());
    }

    #[test]
    fn unknown_and_truncated_reports_are_rejected() {
        let parser = IntuosProParser::new();
        assert!(parser.parse(&[]).is_none());
        assert!(parser.parse(&[0x0C, 0, 0, 0, 1]).is_none());
        assert!(parser.parse(&[0x03, 0, 0]).is_none());
    }
}
