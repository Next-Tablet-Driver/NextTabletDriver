use crate::TabletData;
use crate::parsers::ReportParser;

/// Parser for the auxiliary reports of Wacom Bamboo V2 tablets, a port of the
/// `BambooV2AuxReportParser` of `OpenTabletDriver`.
pub struct BambooV2AuxParser;

impl ReportParser for BambooV2AuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, b1, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::TabletStatus::Aux,
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
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;
    use crate::TabletStatus;

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
