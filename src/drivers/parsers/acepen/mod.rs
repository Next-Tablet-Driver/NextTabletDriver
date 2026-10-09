use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;
use std::sync::atomic::{AtomicU8, Ordering};

pub struct AcepenParser {
    aux_state: AtomicU8,
}

impl AcepenParser {
    #[must_use]
    pub const fn new() -> Self {
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
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));
                let pressure = u16::from_le_bytes([*p_lo, *p_hi]);

                let mut buttons: u8 = 0;
                if (*b2 & 0x02) != 0 {
                    buttons |= 1 << 0;
                }
                if (*b2 & 0x04) != 0 {
                    buttons |= 1 << 1;
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
                    tilt_x: t_x.cast_signed(),
                    tilt_y: t_y.cast_signed(),
                    buttons,
                    is_connected: true,
                    ..Default::default()
                };
                tablet_data.set_raw(data);
                Some(tablet_data)
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

                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: current_state,
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn test_acepen_pen_contact() -> Result<(), Box<dyn std::error::Error>> {
        let parser = AcepenParser::new();
        let data: [u8; 11] = [0, 0x41, 0xA2, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00, 10, 20];
        let report = parser
            .parse(&data)
            .ok_or("Acepen parser failed to parse pen packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
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
        assert_eq!(report.status, crate::drivers::TabletStatus::Aux);
        assert_eq!(report.buttons, 4);
        Ok(())
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;
        use crate::drivers::TabletStatus;

        fn pen(b2: u8, pressure: u16) -> [u8; 11] {
            let p = pressure.to_le_bytes();
            [0, 0x41, b2, 0x02, 0x01, 0x04, 0x03, p[0], p[1], 0xFE, 0x05]
        }

        #[test]
        fn a_pen_without_pressure_hovers_and_reports_both_barrel_buttons() {
            let parsed = AcepenParser::default().parse(&pen(0xA6, 0)).unwrap();
            assert_eq!(parsed.status, TabletStatus::Hover);
            assert_eq!(parsed.buttons, 0b11);
            assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 5));
        }

        #[test]
        fn the_barrel_button_bit_alone_is_the_second_button() {
            assert_eq!(
                AcepenParser::new().parse(&pen(0xA4, 1)).unwrap().buttons,
                0b10
            );
        }

        #[test]
        fn aux_keys_are_latched_until_they_are_released() {
            let parser = AcepenParser::new();
            let key = |down: u8, mask: u8| [0, 0x42, 0, down, mask, 0, 0, 0, 0, 0, 0];
            assert_eq!(parser.parse(&key(1, 0x01)).unwrap().buttons, 0b001);
            assert_eq!(parser.parse(&key(1, 0x04)).unwrap().buttons, 0b101);
            // Releasing a key clears only its own bit.
            assert_eq!(parser.parse(&key(0, 0x01)).unwrap().buttons, 0b100);
            assert_eq!(parser.parse(&key(0, 0x04)).unwrap().buttons, 0);
        }

        #[test]
        fn an_aux_report_without_a_mask_addresses_the_first_key() {
            let parser = AcepenParser::new();
            let parsed = parser.parse(&[0, 0x42, 0, 1, 0]).unwrap();
            assert_eq!(parsed.buttons, 1);
        }

        #[test]
        fn other_report_kinds_are_ignored() {
            let parser = AcepenParser::new();
            assert!(parser.parse(&[0, 0x99, 0, 0, 0]).is_none());
            assert!(parser.parse(&[]).is_none());
            // A pen report whose mode nibble is not 0xA is not a pen report.
            assert!(parser.parse(&pen(0x12, 5)).is_none());
        }
    }
}
