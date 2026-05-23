use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct Wacom64bAuxParser;

impl ReportParser for Wacom64bAuxParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, _, 0x81, ..] => None,
            [_, n_chunks, rest @ ..] => {
                let mut buttons: u8 = 0;
                let n = *n_chunks as usize;

                // Process chunks of 8 bytes starting from index 2
                for chunk in rest.chunks_exact(8).take(n) {
                    if let [id, aux_byte, ..] = chunk
                        && *id == 0x80
                    {
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

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons,
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
