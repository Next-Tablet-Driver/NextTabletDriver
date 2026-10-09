use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct GraphireParser;

impl ReportParser for GraphireParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi_aux, ..] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
                let pressure_val = u16::from(*p_lo) | (u16::from(*p_hi_aux & 0x03) << 8);

                let pos_available = (*b1 & 0x80) != 0
                    || *x_lo != 0
                    || *x_hi != 0
                    || *y_lo != 0
                    || *y_hi != 0
                    || pressure_val != 0;

                if pos_available {
                    if (*b1 & 0x40) != 0 {
                        // Mouse Report
                        let mut buttons: u8 = 0;
                        if (*p_hi_aux & 0x40) != 0 {
                            buttons |= 1 << 0;
                        }
                        if (*p_hi_aux & 0x80) != 0 {
                            buttons |= 1 << 1;
                        }

                        let mut tablet_data = TabletData {
                            status: crate::drivers::TabletStatus::Mouse,
                            x,
                            y,
                            buttons,
                            is_connected: true,
                            ..Default::default()
                        };
                        tablet_data.set_raw(data);
                        return Some(tablet_data);
                    }

                    // Tablet Report
                    let pressure = if (*b1 & 0x01) != 0 { pressure_val } else { 0 };
                    let eraser = (*b1 & 0x20) != 0;

                    let mut buttons: u8 = 0;
                    if (*b1 & 0x02) != 0 {
                        buttons |= 1 << 0;
                    }
                    if (*b1 & 0x04) != 0 {
                        buttons |= 1 << 1;
                    }
                    if (*p_hi_aux & 0x40) != 0 {
                        buttons |= 1 << 2;
                    }
                    if (*p_hi_aux & 0x80) != 0 {
                        buttons |= 1 << 3;
                    }

                    let status = if pressure > 0 {
                        crate::drivers::TabletStatus::Contact
                    } else {
                        crate::drivers::TabletStatus::Hover
                    };

                    let mut tablet_data = TabletData {
                        status,
                        x,
                        y,
                        pressure,
                        buttons,
                        eraser,
                        is_connected: true,
                        ..Default::default()
                    };
                    tablet_data.set_raw(data);
                    Some(tablet_data)
                } else {
                    // Aux Report
                    let mut buttons: u8 = 0;
                    if (*p_hi_aux & 0x40) != 0 {
                        buttons |= 1 << 0;
                    }
                    if (*p_hi_aux & 0x80) != 0 {
                        buttons |= 1 << 1;
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

    /// `[id, b1, x, y, p_lo, p_hi_aux]` with 16-bit little-endian coordinates.
    fn report(b1: u8, x: u16, y: u16, p_lo: u8, p_hi_aux: u8) -> [u8; 8] {
        let (x, y) = (x.to_le_bytes(), y.to_le_bytes());
        [0x02, b1, x[0], x[1], y[0], y[1], p_lo, p_hi_aux]
    }

    #[test]
    fn pen_contact_decodes_position_and_ten_bit_pressure() {
        let data = report(0x81, 0x1234, 0x0567, 0x55, 0x02);
        let parsed = GraphireParser.parse(&data).unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!((parsed.x, parsed.y), (0x1234, 0x0567));
        assert_eq!(parsed.pressure, 0x255);
        assert_eq!(parsed.buttons, 0);
        assert!(!parsed.eraser);
        assert!(parsed.is_connected);
        assert_eq!(parsed.raw_len, 8);
    }

    #[test]
    fn pressure_is_ignored_while_the_tip_flag_is_clear() {
        let data = report(0x80, 100, 200, 0xFF, 0x03);
        let parsed = GraphireParser.parse(&data).unwrap();
        assert_eq!(parsed.status, TabletStatus::Hover);
        assert_eq!(parsed.pressure, 0);
    }

    #[test]
    fn eraser_flag_and_all_four_buttons() {
        // b1: in range + tip + eraser + both barrel buttons; aux bits add buttons 2 and 3.
        let data = report(0x80 | 0x20 | 0x01 | 0x02 | 0x04, 1, 1, 1, 0xC0);
        let parsed = GraphireParser.parse(&data).unwrap();
        assert!(parsed.eraser);
        assert_eq!(parsed.buttons, 0b1111);
    }

    #[test]
    fn mouse_reports_carry_two_buttons_and_no_pressure() {
        let parsed = GraphireParser
            .parse(&report(0xC0, 10, 20, 0, 0x40))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Mouse);
        assert_eq!((parsed.x, parsed.y, parsed.pressure), (10, 20, 0));
        assert_eq!(parsed.buttons, 0b01);

        let parsed = GraphireParser
            .parse(&report(0xC0, 10, 20, 0, 0xC0))
            .unwrap();
        assert_eq!(parsed.buttons, 0b11);
    }

    #[test]
    fn empty_position_means_an_aux_report() {
        let parsed = GraphireParser.parse(&report(0x00, 0, 0, 0, 0x80)).unwrap();
        assert_eq!(parsed.status, TabletStatus::Aux);
        assert_eq!(parsed.buttons, 0b10);
        assert_eq!((parsed.x, parsed.y), (0, 0));
    }

    #[test]
    fn rejects_other_report_ids_and_short_reports() {
        let mut data = report(0x81, 1, 1, 1, 0);
        data[0] = 0x03;
        assert!(GraphireParser.parse(&data).is_none());
        assert!(GraphireParser.parse(&data[..7]).is_none());
        assert!(GraphireParser.parse(&[]).is_none());
    }
}
