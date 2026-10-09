use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;
use crate::engine::state::{LockRecoveryExt, WriteRecoverExt};
use std::sync::Mutex;

// Intuos V1

pub struct IntuosV1Parser {
    pressure: Mutex<u16>,
    tilt_x: Mutex<i8>,
    tilt_y: Mutex<i8>,
    buttons: Mutex<u8>,
}

impl IntuosV1Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pressure: Mutex::new(0),
            tilt_x: Mutex::new(0),
            tilt_y: Mutex::new(0),
            buttons: Mutex::new(0),
        }
    }
}

impl Default for IntuosV1Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl IntuosV1Parser {
    pub(crate) fn parse_internal(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02 | 0x10, ..] => self.parse_tool(data),
            [0x03, ..] => Self::parse_aux(data),
            _ => None,
        }
    }

    fn parse_tool(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x10, 0x20, ..] | [_, 0x80, ..] => None,
            [_id, b1, b2, b3, b4, b5, b6, b7, b8, b9, ..] => {
                let is_rotation = (*b1 & 0x02) != 0 && (*b1 & 0x08) != 0;
                let is_tablet = (*b1 & 0x20) != 0;

                if is_tablet {
                    let x = u32::from(
                        ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1),
                    );
                    let y = u32::from(
                        ((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1),
                    );

                    let tilt_x = (i16::from(((*b7 << 1) & 0x7E) | (*b8 >> 7)) - 64) as i8;
                    let tilt_y = (i16::from(*b8 & 0x7F) - 64) as i8;

                    let pressure =
                        (u16::from(*b6) << 3) | u16::from((*b7 & 0xC0) >> 5) | u16::from(*b1 & 1);

                    let mut buttons: u8 = 0;
                    if (*b1 & 0x02) != 0 {
                        buttons |= 1 << 0;
                    }
                    if (*b1 & 0x04) != 0 {
                        buttons |= 1 << 1;
                    }

                    *self.pressure.lock().unwrap_or_reset("wacom_prev_pressure") = pressure;
                    *self.tilt_x.lock().unwrap_or_reset("wacom_prev_tilt_x") = tilt_x;
                    *self.tilt_y.lock().unwrap_or_reset("wacom_tilt_y") = tilt_y;
                    *self.buttons.lock().unwrap_or_reset("wacom_buttons") = buttons;

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
                        tilt_x,
                        tilt_y,
                        buttons,
                        hover_distance: *b9,
                        is_connected: true,
                        ..Default::default()
                    };
                    tablet_data.set_raw(data);
                    Some(tablet_data)
                } else if is_rotation {
                    let x = u32::from(
                        ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1),
                    );
                    let y = u32::from(
                        ((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1),
                    );

                    let mut tablet_data = TabletData {
                        status: crate::drivers::TabletStatus::Rotation,
                        x,
                        y,
                        pressure: *self.pressure.lock().unwrap_or_log("wacom_prev_pressure"),
                        tilt_x: *self.tilt_x.lock().unwrap_or_log("wacom_prev_tilt_x"),
                        tilt_y: *self.tilt_y.lock().unwrap_or_log("wacom_tilt_y"),
                        buttons: *self.buttons.lock().unwrap_or_log("wacom_buttons"),
                        hover_distance: *b9,
                        is_connected: true,
                        ..Default::default()
                    };
                    tablet_data.set_raw(data);
                    Some(tablet_data)
                } else if *b1 == 0xC2 {
                    let eraser = (*b3 & 0x80) != 0;
                    let mut tablet_data = TabletData {
                        status: crate::drivers::TabletStatus::Tool,
                        eraser,
                        is_connected: true,
                        ..Default::default()
                    };
                    tablet_data.set_raw(data);
                    Some(tablet_data)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn parse_aux(data: &[u8]) -> Option<TabletData> {
        match data {
            [_, _, _, _, b4, ..] => {
                let mut tablet_data = TabletData {
                    status: crate::drivers::TabletStatus::Aux,
                    buttons: *b4,
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

impl ReportParser for IntuosV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        self.parse_internal(data)
    }
}

pub struct WacomDriverIntuosV1Parser {
    inner: IntuosV1Parser,
}

impl WacomDriverIntuosV1Parser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: IntuosV1Parser::new(),
        }
    }
}

impl Default for WacomDriverIntuosV1Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportParser for WacomDriverIntuosV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, rest @ ..] => self.inner.parse_internal(rest),
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
    fn test_intuos_v1_tablet_report() -> Result<(), Box<dyn std::error::Error>> {
        let parser = IntuosV1Parser::new();
        // Report ID 0x02, status with bit 5 set (tablet), X/Y/Pressure/Tilt
        let data = [
            0x02, // ID
            0x20, // Status (tablet)
            0x12, 0x34, // X
            0x56, 0x78, // Y
            0x80, // Pressure mid
            0x40, // Pressure high bits + Tilt
            0x40, // Tilt
            0x00, // Hover distance/coord low bit
        ];
        let result = parser
            .parse(&data)
            .ok_or("Intuos V1 parser failed to parse tablet report")?;
        assert_eq!(result.status, crate::drivers::TabletStatus::Contact);
        Ok(())
    }

    const TOOL: [u8; 10] = [0x02, 0x27, 0x12, 0x34, 0x56, 0x78, 0x40, 0x80, 0x80, 0x03];

    #[test]
    fn tablet_report_decodes_every_field() {
        let parsed = IntuosV1Parser::new().parse(&TOOL).unwrap();
        assert_eq!(parsed.status, crate::drivers::TabletStatus::Contact);
        assert_eq!((parsed.x, parsed.y), (0x2469, 0xACF1));
        assert_eq!(parsed.pressure, 0x200 | 4 | 1);
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (-63, -64));
        assert_eq!(parsed.buttons, 0b11);
        assert_eq!(parsed.hover_distance, 3);
        assert_eq!(parsed.raw_len, 10);
    }

    #[test]
    fn zero_pressure_is_hover() {
        let mut data = TOOL;
        data[1] = 0x20;
        data[6] = 0;
        data[7] = 0;
        let parsed = IntuosV1Parser::new().parse(&data).unwrap();
        assert_eq!(parsed.status, crate::drivers::TabletStatus::Hover);
        assert_eq!(parsed.pressure, 0);
    }

    #[test]
    fn rotation_reports_reuse_the_last_pen_state() {
        let parser = IntuosV1Parser::new();
        parser.parse(&TOOL).unwrap();
        let mut rotation = TOOL;
        rotation[1] = 0x0A;
        let parsed = parser.parse(&rotation).unwrap();
        assert_eq!(parsed.status, crate::drivers::TabletStatus::Rotation);
        assert_eq!((parsed.x, parsed.y), (0x2469, 0xACF1));
        assert_eq!(parsed.pressure, 0x205);
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (-63, -64));
        assert_eq!(parsed.buttons, 0b11);
    }

    #[test]
    fn tool_change_reports_flag_the_eraser_end() {
        let mut data = [0x02, 0xC2, 0, 0x80, 0, 0, 0, 0, 0, 0];
        let eraser = IntuosV1Parser::new().parse(&data).unwrap();
        assert_eq!(eraser.status, crate::drivers::TabletStatus::Tool);
        assert!(eraser.eraser);
        data[3] = 0;
        assert!(!IntuosV1Parser::new().parse(&data).unwrap().eraser);
    }

    #[test]
    fn aux_report_exposes_byte_4_as_buttons() {
        let parsed = IntuosV1Parser::new().parse(&[0x03, 0, 0, 0, 0x09]).unwrap();
        assert_eq!(parsed.status, crate::drivers::TabletStatus::Aux);
        assert_eq!(parsed.buttons, 9);
    }

    #[test]
    fn ignored_and_malformed_reports() {
        let parser = IntuosV1Parser::new();
        // Proximity-out markers and unknown report kinds produce nothing.
        assert!(
            parser
                .parse(&[0x10, 0x20, 0, 0, 0, 0, 0, 0, 0, 0])
                .is_none()
        );
        assert!(
            parser
                .parse(&[0x02, 0x80, 0, 0, 0, 0, 0, 0, 0, 0])
                .is_none()
        );
        assert!(
            parser
                .parse(&[0x02, 0x00, 0, 0, 0, 0, 0, 0, 0, 0])
                .is_none()
        );
        assert!(
            parser
                .parse(&[0x55, 0x27, 0, 0, 0, 0, 0, 0, 0, 0])
                .is_none()
        );
        // A pen report shorter than its fixed layout is rejected, not misread.
        assert!(parser.parse(&TOOL[..9]).is_none());
        assert!(parser.parse(&[]).is_none());
    }

    #[test]
    fn driver_variant_skips_the_leading_byte() {
        let mut framed = vec![0xFF];
        framed.extend_from_slice(&TOOL);
        let parsed = WacomDriverIntuosV1Parser::new().parse(&framed).unwrap();
        assert_eq!((parsed.x, parsed.y), (0x2469, 0xACF1));
        assert!(WacomDriverIntuosV1Parser::default().parse(&[]).is_none());
    }
}
