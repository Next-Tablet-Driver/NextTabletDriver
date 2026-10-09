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
                for chunk in rest.as_chunks::<8>().0.iter().take(n) {
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    /// `[_, n_chunks]` followed by 8-byte chunks `[id, aux, 0 x 6]`.
    fn report(chunks: &[(u8, u8)]) -> Vec<u8> {
        let mut data = vec![0x00, u8::try_from(chunks.len()).unwrap()];
        for (id, aux) in chunks {
            data.extend_from_slice(&[*id, *aux, 0, 0, 0, 0, 0, 0]);
        }
        data
    }

    #[test]
    fn aux_buttons_of_every_0x80_chunk_are_merged() {
        let parsed = Wacom64bAuxParser
            .parse(&report(&[(0x80, 0x05), (0x80, 0x0A)]))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0b1111);
        assert!(parsed.is_connected);
    }

    #[test]
    fn chunks_with_another_id_are_ignored() {
        let parsed = Wacom64bAuxParser
            .parse(&report(&[(0x80, 0x01), (0x40, 0x0F)]))
            .unwrap();
        assert_eq!(parsed.buttons, 0b0001);
    }

    #[test]
    fn only_the_announced_number_of_chunks_is_read() {
        let mut data = report(&[(0x80, 0x01), (0x80, 0x08)]);
        data[1] = 1;
        assert_eq!(Wacom64bAuxParser.parse(&data).unwrap().buttons, 0b0001);
    }

    #[test]
    fn a_0x81_first_chunk_is_not_an_aux_report() {
        assert!(Wacom64bAuxParser.parse(&report(&[(0x81, 0x0F)])).is_none());
    }

    #[test]
    fn rejects_reports_too_short_to_hold_a_header() {
        assert!(Wacom64bAuxParser.parse(&[]).is_none());
        assert!(Wacom64bAuxParser.parse(&[0x00]).is_none());
    }
}
