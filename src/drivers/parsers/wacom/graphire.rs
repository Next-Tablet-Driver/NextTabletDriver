use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct GraphireParser;

impl ReportParser for GraphireParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
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
