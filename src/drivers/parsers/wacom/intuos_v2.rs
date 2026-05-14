use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos V2

pub struct IntuosV2Parser;

impl IntuosV2Parser {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for IntuosV2Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl IntuosV2Parser {
    fn parse_internal(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [
                0x10,
                b1,
                x_lo,
                _,
                x_hi,
                y_lo,
                _,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                t_y,
                _,
                _,
                _,
                _,
                h_dist,
                ..,
            ] => {
                let x = (*x_lo as u32) | ((*x_hi as u32) << 16);
                let y = (*y_lo as u32) | ((*y_hi as u32) << 16);
                let pressure = (*p_lo as u16) | ((*p_hi as u16) << 8);

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                let eraser = (*b1 & 0x10) != 0;

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x: x.min(0xFFFF) as u16,
                    y: y.min(0xFFFF) as u16,
                    pressure,
                    tilt_x: *t_x as i8,
                    tilt_y: *t_y as i8,
                    buttons,
                    eraser,
                    hover_distance: *h_dist,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            [
                0x1E,
                b1,
                _,
                x_lo,
                _,
                x_hi,
                y_lo,
                _,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                t_y,
                ..,
            ] => {
                let x = (*x_lo as u32) | ((*x_hi as u32) << 16);
                let y = (*y_lo as u32) | ((*y_hi as u32) << 16);
                let pressure = (*p_lo as u16) | ((*p_hi as u16) << 8);

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                if (*b1 & 0x08) != 0 {
                    buttons |= 1 << 2;
                }
                let eraser = (*b1 & 0x10) != 0;

                let status = if pressure > 0 { "Contact" } else { "Hover" };
                // hover_distance for offset report is also at index 11 (t_x) in original code?
                // data[11] was tx. Original code: let hover_distance = if offset { data[11] } else { data[16] };
                // So h_dist is same as t_x here.

                Some(TabletData {
                    status: status.to_string(),
                    x: x.min(0xFFFF) as u16,
                    y: y.min(0xFFFF) as u16,
                    pressure,
                    tilt_x: *t_x as i8,
                    tilt_y: *t_y as i8,
                    buttons,
                    eraser,
                    hover_distance: *t_x,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            [0x11, b1, ..] => Some(TabletData {
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

impl ReportParser for IntuosV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.parse_internal(data, raw)
    }
}

pub struct WacomDriverIntuosV2Parser {
    inner: IntuosV2Parser,
}

impl WacomDriverIntuosV2Parser {
    pub const fn new() -> Self {
        Self {
            inner: IntuosV2Parser::new(),
        }
    }
}

impl Default for WacomDriverIntuosV2Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuosV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

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
    fn test_intuos_v2_tablet_report() {
        let parser = IntuosV2Parser::new();
        // Report ID 0x10, X/Y/Pressure/Tilt/Buttons
        let mut data = [0u8; 17];
        data[0] = 0x10;
        data[1] = 0x02; // Pen Button 1
        data[2] = 0x34; // X low
        data[4] = 0x12; // X high
        data[8] = 0xFF; // Pressure low
        data[9] = 0x03; // Pressure high

        let result = parser.parse(&data).expect("Should parse");
        assert_eq!(result.buttons, 1 << 0);
        assert_eq!(result.pressure, 1023);
    }
}
