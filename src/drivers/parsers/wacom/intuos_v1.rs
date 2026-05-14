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
    pub(crate) fn parse_internal(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [0x02 | 0x10, ..] => self.parse_tool(data, raw),
            [0x03, ..] => Self::parse_aux(data, raw),
            _ => None,
        }
    }

    fn parse_tool(&self, data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [0x10, 0x20, ..] | [_, 0x80, ..] => None,
            [_id, b1, b2, b3, b4, b5, b6, b7, b8, b9, ..] => {
                let is_rotation = (*b1 & 0x02) != 0 && (*b1 & 0x08) != 0;
                let is_tablet = (*b1 & 0x20) != 0;

                if is_tablet {
                    let x =
                        ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1);
                    let y = ((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1);

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

                    let status = if pressure > 0 { "Contact" } else { "Hover" };

                    Some(TabletData {
                        status: status.to_string(),
                        x,
                        y,
                        pressure,
                        tilt_x,
                        tilt_y,
                        buttons,
                        hover_distance: *b9,
                        raw_data: raw,
                        is_connected: true,
                        ..Default::default()
                    })
                } else if is_rotation {
                    let x =
                        ((u16::from(*b2) << 8) | u16::from(*b3)) << 1 | u16::from((*b9 >> 1) & 1);
                    let y = ((u16::from(*b4) << 8) | u16::from(*b5)) << 1 | u16::from(*b9 & 1);

                    Some(TabletData {
                        status: "Rotation".to_string(),
                        x,
                        y,
                        pressure: *self.pressure.lock().unwrap_or_log("wacom_prev_pressure"),
                        tilt_x: *self.tilt_x.lock().unwrap_or_log("wacom_prev_tilt_x"),
                        tilt_y: *self.tilt_y.lock().unwrap_or_log("wacom_tilt_y"),
                        buttons: *self.buttons.lock().unwrap_or_log("wacom_buttons"),
                        hover_distance: *b9,
                        raw_data: raw,
                        is_connected: true,
                        ..Default::default()
                    })
                } else if *b1 == 0xC2 {
                    let eraser = (*b3 & 0x80) != 0;
                    Some(TabletData {
                        status: "Tool".to_string(),
                        eraser,
                        raw_data: raw,
                        is_connected: true,
                        ..Default::default()
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn parse_aux(data: &[u8], raw: String) -> Option<TabletData> {
        match data {
            [_, _, _, _, b4, ..] => Some(TabletData {
                status: "Aux".to_string(),
                buttons: *b4,
                raw_data: raw,
                is_connected: true,
                ..Default::default()
            }),
            _ => None,
        }
    }
}

impl ReportParser for IntuosV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.parse_internal(data, raw)
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
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
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
        assert_eq!(result.status, "Contact");
        Ok(())
    }
}
