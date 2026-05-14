use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos V3

pub struct IntuosV3Parser;

impl IntuosV3Parser {
    #[must_use] 
    pub const fn new() -> Self {
        Self
    }
}

impl Default for IntuosV3Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl IntuosV3Parser {
    fn parse_internal(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [0x11, ..] => self.parse_aux(data, raw),
            [0x1E, ..] => self.parse_extended(data, raw),
            [0x1F, 0x01, ..] => self.parse_tablet(data, raw),
            _ => None,
        }
    }

    fn parse_tablet(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [
                _,
                _,
                b2,
                x_lo,
                x_hi,
                y_lo,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                _,
                t_y,
                _,
                h_dist,
                ..,
            ] => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let tilt_x = if (*t_x & 0x80) != 0 {
                    (i16::from(*t_x) - 0xFF) as i8
                } else {
                    *t_x as i8
                };
                let tilt_y = if (*t_y & 0x80) != 0 {
                    (i16::from(*t_y) - 0xFF) as i8
                } else {
                    *t_y as i8
                };

                let mut buttons: u8 = 0;
                if (*b2 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b2 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                let eraser = (*b2 & 0x20) != 0;

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x,
                    y,
                    pressure,
                    tilt_x,
                    tilt_y,
                    buttons,
                    eraser,
                    hover_distance: *h_dist,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }

    fn parse_extended(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [
                _,
                _,
                b2,
                x_lo,
                x_hi,
                x_ext,
                y_lo,
                y_hi,
                y_ext,
                p_lo,
                p_hi,
                t_x_lo,
                t_x_hi,
                t_y_lo,
                t_y_hi,
                _,
                _,
                _,
                _,
                h_dist,
                ..,
            ] => {
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi])) | (u32::from(*x_ext) << 16);
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi])) | (u32::from(*y_ext) << 16);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);
                let tilt_x = (i16::from_le_bytes([*t_x_lo, *t_x_hi])) as i8;
                let tilt_y = (i16::from_le_bytes([*t_y_lo, *t_y_hi])) as i8;

                let mut buttons: u8 = 0;
                if (*b2 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b2 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b2 & 0x08) != 0 {
                    buttons |= 1 << 2;
                }
                let eraser = (*b2 & 0x20) != 0;

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x: x.min(0xFFFF) as u16,
                    y: y.min(0xFFFF) as u16,
                    pressure,
                    tilt_x,
                    tilt_y,
                    buttons,
                    eraser,
                    hover_distance: *h_dist,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }

    fn parse_aux(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [_, b1, _, b2, ..] => {
                let mut buttons: u16 = 0;
                if (*b1 & 1) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 2) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b1 & 4) != 0 {
                    buttons |= 1 << 2;
                }
                if (*b1 & 8) != 0 {
                    buttons |= 1 << 3;
                }
                if (*b2 & 1) != 0 {
                    buttons |= 1 << 4;
                }
                if (*b1 & 16) != 0 {
                    buttons |= 1 << 5;
                }
                if (*b1 & 32) != 0 {
                    buttons |= 1 << 6;
                }
                if (*b1 & 64) != 0 {
                    buttons |= 1 << 7;
                }

                Some(TabletData {
                    status: "Aux".to_string(),
                    buttons: buttons as u8,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}

impl ReportParser for IntuosV3Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.parse_internal(data, raw)
    }
}

pub struct WacomDriverIntuosV3Parser {
    inner: IntuosV3Parser,
}

impl WacomDriverIntuosV3Parser {
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            inner: IntuosV3Parser::new(),
        }
    }
}

impl Default for WacomDriverIntuosV3Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuosV3Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        // Skip the first byte safely
        match data {
            [_, rest @ ..] => self.inner.parse_internal(rest, raw),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intuos_v3_tablet_report() {
        let parser = IntuosV3Parser::new();
        // Report ID 1F, status 01, Pen buttons, X, Y, Pressure, Tilt
        let mut data = [0u8; 15];
        data[0] = 0x1F;
        data[1] = 0x01;
        data[2] = 0x02; // Pen button 1
        data[3] = 0x01;
        data[4] = 0x00; // X = 1
        data[7] = 0xAA;
        data[8] = 0x00; // Pressure = 170

        let result = parser.parse(&data).expect("Should parse");
        assert_eq!(result.x, 1);
        assert_eq!(result.pressure, 170);
        assert_eq!(result.buttons, 1 << 0);
    }
}
