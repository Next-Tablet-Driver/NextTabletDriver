use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct GraphireParser;

impl ReportParser for GraphireParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [0x02, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi_aux, ..] => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
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

                        return Some(TabletData {
                            status: "Mouse".to_string(),
                            x,
                            y,
                            buttons,
                            raw_data: raw,
                            is_connected: true,
                            ..Default::default()
                        });
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

                    let status = if pressure > 0 { "Contact" } else { "Hover" };

                    Some(TabletData {
                        status: status.to_string(),
                        x,
                        y,
                        pressure,
                        buttons,
                        eraser,
                        raw_data: raw,
                        is_connected: true,
                        ..Default::default()
                    })
                } else {
                    // Aux Report
                    let mut buttons: u8 = 0;
                    if (*p_hi_aux & 0x40) != 0 {
                        buttons |= 1 << 0;
                    }
                    if (*p_hi_aux & 0x80) != 0 {
                        buttons |= 1 << 1;
                    }

                    Some(TabletData {
                        status: "Aux".to_string(),
                        buttons,
                        raw_data: raw,
                        is_connected: true,
                        ..Default::default()
                    })
                }
            }
            _ => None,
        }
    }
}
