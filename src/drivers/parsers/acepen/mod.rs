use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;
use std::sync::atomic::{AtomicU8, Ordering};

pub struct AcepenParser {
    aux_state: AtomicU8,
}

impl AcepenParser {
    pub fn new() -> Self {
        Self {
            aux_state: AtomicU8::new(0),
        }
    }
}

impl Default for AcepenParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for AcepenParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            // PEN_MODE
            [
                _,
                0x41,
                b2,
                x_lo,
                x_hi,
                y_lo,
                y_hi,
                p_lo,
                p_hi,
                t_x,
                t_y,
                ..,
            ] if (*b2 & 0xF0) == 0xA0 => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*b2 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b2 & 0x04) != 0 {
                    buttons |= 1 << 1;
                }

                let status = if pressure > 0 { "Contact" } else { "Hover" };

                Some(TabletData {
                    status: status.to_string(),
                    x,
                    y,
                    pressure,
                    tilt_x: *t_x as i8,
                    tilt_y: *t_y as i8,
                    buttons,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            // AUX_MODE
            [_, 0x42, _, b3, b4, ..] => {
                let bit_index = if *b4 > 0 { b4.trailing_zeros() } else { 0 };
                let is_set = (*b3 & 0x01) != 0;

                let mut current_state = self.aux_state.load(Ordering::Relaxed);
                if is_set {
                    current_state |= 1 << bit_index;
                } else {
                    current_state &= !(1 << bit_index);
                }
                self.aux_state.store(current_state, Ordering::Relaxed);

                Some(TabletData {
                    status: "Aux".to_string(),
                    buttons: current_state,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acepen_pen_contact() -> Result<(), Box<dyn std::error::Error>> {
        let parser = AcepenParser::new();
        let data: [u8; 11] = [0, 0x41, 0xA2, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00, 10, 20];
        let report = parser
            .parse(&data)
            .ok_or("Acepen parser failed to parse pen packet")?;
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        assert_eq!(report.y, 772);
        assert_eq!(report.pressure, 1);
        assert_eq!(report.buttons, 1);
        assert_eq!(report.tilt_x, 10);
        assert_eq!(report.tilt_y, 20);
        Ok(())
    }

    #[test]
    fn test_acepen_aux() -> Result<(), Box<dyn std::error::Error>> {
        let parser = AcepenParser::new();
        let data: [u8; 11] = [0, 0x42, 0, 1, 4, 0, 0, 0, 0, 0, 0];
        let report = parser
            .parse(&data)
            .ok_or("Acepen parser failed to parse aux packet")?;
        assert_eq!(report.status, "Aux");
        assert_eq!(report.buttons, 4);
        Ok(())
    }
}
