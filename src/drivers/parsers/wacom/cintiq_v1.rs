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
