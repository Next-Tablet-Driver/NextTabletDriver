use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

// Intuos V2

pub struct IntuosV2Parser;

impl IntuosV2Parser {
    #[must_use]
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
    fn parse_internal(data: &[u8]) -> Option<TabletData> {
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
                let x = u32::from(*x_lo) | (u32::from(*x_hi) << 16);
                let y = u32::from(*y_lo) | (u32::from(*y_hi) << 16);
                let pressure = u16::from(*p_lo) | (u16::from(*p_hi) << 8);

                let mut buttons: u8 = 0;
                if (*b1 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b1 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }
                let eraser = (*b1 & 0x10) != 0;

                let status = if pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else {
                    crate::drivers::TabletStatus::Hover
                };

                let mut tablet_data = TabletData {
                    status,
                    x: x.min(0xFFFF) as u16,
                    y: y.min(0xFFFF) as u16,
                    pressure,
                    tilt_x: t_x.cast_signed(),
                    tilt_y: t_y.cast_signed(),
                    buttons,
                    eraser,
                    hover_distance: *h_dist,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
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
                let x = u32::from(*x_lo) | (u32::from(*x_hi) << 16);
                let y = u32::from(*y_lo) | (u32::from(*y_hi) << 16);
                let pressure = u16::from(*p_lo) | (u16::from(*p_hi) << 8);

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

                let status = if pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else {
                    crate::drivers::TabletStatus::Hover
                };

                let mut tablet_data = TabletData {
                    status,
                    x: x.min(0xFFFF) as u16,
                    y: y.min(0xFFFF) as u16,
                    pressure,
                    tilt_x: t_x.cast_signed(),
                    tilt_y: t_y.cast_signed(),
                    buttons,
                    eraser,
                    hover_distance: *t_x,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
            }
            [0x11, b1, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
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

impl ReportParser for IntuosV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        Self::parse_internal(data)
    }
}

pub struct WacomDriverIntuosV2Parser;

impl WacomDriverIntuosV2Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for WacomDriverIntuosV2Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuosV2Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, rest @ ..] => IntuosV2Parser::parse_internal(rest),
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
