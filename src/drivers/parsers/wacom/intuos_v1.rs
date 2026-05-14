use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;
use crate::engine::state::{LockRecoveryExt, WriteRecoverExt};
use std::sync::Mutex;

// Intuos V1

pub struct IntuosV1Parser {
    prev_pressure: Mutex<u16>,
    prev_tilt_x: Mutex<i8>,
    prev_tilt_y: Mutex<i8>,
    prev_buttons: Mutex<u8>,
}

impl IntuosV1Parser {
    pub fn new() -> Self {
        Self {
            prev_pressure: Mutex::new(0),
            prev_tilt_x: Mutex::new(0),
            prev_tilt_y: Mutex::new(0),
            prev_buttons: Mutex::new(0),
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
        match data[0] {
            0x02 | 0x10 => self.parse_tool(data, raw),
            0x03 => self.parse_aux(data, raw),
            _ => None,
        }
    }

    fn parse_tool(&self, data: &[u8], raw: String) -> Option<TabletData> {
        if data.len() < 10 {
            return None;
        }
        if data[0] == 0x10 && data[1] == 0x20 {
            return None;
        }
        if data[1] == 0x80 {
            return None;
        } // Out of range

        let is_rotation = (data[1] & 0x02) != 0 && (data[1] & 0x08) != 0;
        let is_tablet = (data[1] & 0x20) != 0;

        if is_tablet {
            let x =
                (((data[2] as u16) << 8) | (data[3] as u16)) << 1 | (((data[9] >> 1) & 1) as u16);
            let y = (((data[4] as u16) << 8) | (data[5] as u16)) << 1 | ((data[9] & 1) as u16);

            let tilt_x = ((((data[7] << 1) & 0x7E) | (data[8] >> 7)) as i16 - 64) as i8;
            let tilt_y = ((data[8] & 0x7F) as i16 - 64) as i8;

            let pressure =
                ((data[6] as u16) << 3) | (((data[7] & 0xC0) >> 5) as u16) | ((data[1] & 1) as u16);

            let mut buttons: u8 = 0;
            if (data[1] & 0x02) != 0 {
                buttons |= 1 << 0;
            }
            if (data[1] & 0x04) != 0 {
                buttons |= 1 << 1;
            }

            *self.prev_pressure.lock().unwrap_or_reset("wacom_prev_pressure") = pressure;
            *self.prev_tilt_x.lock().unwrap_or_reset("wacom_prev_tilt_x") = tilt_x;
            *self.prev_tilt_y.lock().unwrap_or_reset("wacom_prev_tilt_y") = tilt_y;
            *self.prev_buttons.lock().unwrap_or_reset("wacom_prev_buttons") = buttons;

            let status = if pressure > 0 { "Contact" } else { "Hover" };
            let hover_distance = data[9];

            Some(TabletData {
                status: status.to_string(),
                x,
                y,
                pressure,
                tilt_x,
                tilt_y,
                buttons,
                hover_distance,
                raw_data: raw,
                is_connected: true,
                ..Default::default()
            })
        } else if is_rotation {
            let x =
                (((data[2] as u16) << 8) | (data[3] as u16)) << 1 | (((data[9] >> 1) & 1) as u16);
            let y = (((data[4] as u16) << 8) | (data[5] as u16)) << 1 | ((data[9] & 1) as u16);

            Some(TabletData {
                status: "Rotation".to_string(),
                x,
                y,
                pressure: *self.prev_pressure.lock().unwrap_or_log("wacom_prev_pressure"),
                tilt_x: *self.prev_tilt_x.lock().unwrap_or_log("wacom_prev_tilt_x"),
                tilt_y: *self.prev_tilt_y.lock().unwrap_or_log("wacom_prev_tilt_y"),
                buttons: *self.prev_buttons.lock().unwrap_or_log("wacom_prev_buttons"),
                hover_distance: data[9],
                raw_data: raw,
                is_connected: true,
                ..Default::default()
            })
        } else if data[1] == 0xC2 {
            // Tool Report
            let eraser = (data[3] & 0x80) != 0;
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

    pub(crate) fn parse_aux(&self, data: &[u8], raw: String) -> Option<TabletData> {
        if data.len() < 5 {
            return None;
        }
        let buttons = data[4];
        Some(TabletData {
            status: "Aux".to_string(),
            buttons,
            raw_data: raw,
            is_connected: true,
            ..Default::default()
        })
    }
}

impl ReportParser for IntuosV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        if data.is_empty() {
            return None;
        }
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.parse_internal(data, raw)
    }
}

pub struct WacomDriverIntuosV1Parser {
    inner: IntuosV1Parser,
}

impl WacomDriverIntuosV1Parser {
    pub fn new() -> Self {
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
        if data.len() < 2 {
            return None;
        }
        // WacomDriver usually skips first byte (report ID) then calls base
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.inner.parse_internal(&data[1..], raw)
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
