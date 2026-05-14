use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BambooV2AuxParser;

impl ReportParser for BambooV2AuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [0x02, b1, ..] => Some(TabletData {
                status: "Aux".to_string(),
                buttons: *b1,
                raw_data: raw,
                is_connected: true,
                ..Default::default()
            }),
            _ => None,
        }
    }
}
