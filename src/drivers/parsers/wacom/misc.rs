use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct Wacom64bAuxParser;

impl ReportParser for Wacom64bAuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, _, 0x81, ..] => None,
            [_, n_chunks, rest @ ..] => {
                let mut buttons: u8 = 0;
                let n = *n_chunks as usize;

                // Process chunks of 8 bytes starting from index 2
                for chunk in rest.chunks_exact(8).take(n) {
                    if let [id, aux_byte, ..] = chunk && *id == 0x80 {
                            if (aux_byte & 0x01) != 0 {
                                buttons |= 1 << 0;
                            }
                            if (aux_byte & 0x02) != 0 {
                                buttons |= 1 << 1;
                            }
                            if (aux_byte & 0x04) != 0 {
                                buttons |= 1 << 2;
                            }
                            if (aux_byte & 0x08) != 0 {
                                buttons |= 1 << 3;
                            }
                        }
                    
                }

                Some(TabletData {
                    status: "Aux".to_string(),
                    buttons,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}
