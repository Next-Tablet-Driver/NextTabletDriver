use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BambooV2AuxParser;

impl ReportParser for BambooV2AuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, b1, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: *b1,
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

    #[test]
    fn the_report_is_an_aux_report_carrying_the_button_byte() {
        let parsed = BambooV2AuxParser.parse(&[0x02, 0x0B, 0, 0]).unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0x0B);
        assert!(parsed.is_connected);
        assert_eq!(parsed.raw_len, 4);
    }

    #[test]
    fn other_report_ids_and_short_reports_are_ignored() {
        assert!(BambooV2AuxParser.parse(&[0x03, 0x0B]).is_none());
        assert!(BambooV2AuxParser.parse(&[0x02]).is_none());
        assert!(BambooV2AuxParser.parse(&[]).is_none());
    }
}
