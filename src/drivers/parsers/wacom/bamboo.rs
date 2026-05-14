use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BambooTabletReport {
    pub x: u16,
    pub y: u16,
    pub pressure: u16,
    pub eraser: bool,
    pub near_proximity: bool,
    pub buttons: u8,
    pub aux_buttons: [bool; 4],
}

impl BambooTabletReport {
    #[must_use]
    pub fn new(report: &[u8]) -> Option<Self> {
        match report {
            [_, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi_aux, ..] => {
                let x = u16::from_le_bytes([*x_lo, *x_hi]);
                let y = u16::from_le_bytes([*y_lo, *y_hi]);

                let pressure = if (*b1 & 0x01) != 0 {
                    u16::from(*p_lo) | (u16::from(*p_hi_aux & 0x03) << 8)
                } else {
                    0
                };

                Some(Self {
                    x,
                    y,
                    pressure,
                    eraser: (*b1 & 0x20) != 0,
                    near_proximity: (*b1 & 0x80) != 0,
                    buttons: (*b1 >> 1) & 0x03,
                    aux_buttons: [
                        (*p_hi_aux & 0x08) != 0,
                        (*p_hi_aux & 0x10) != 0,
                        (*p_hi_aux & 0x20) != 0,
                        (*p_hi_aux & 0x40) != 0,
                    ],
                })
            }
            _ => None,
        }
    }
}

pub struct BambooParser;

impl ReportParser for BambooParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x02, ..] => {
                let report = BambooTabletReport::new(data)?;
                let status = if report.pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else if report.near_proximity {
                    crate::drivers::TabletStatus::Hover
                } else {
                    crate::drivers::TabletStatus::OutOfRange
                };

                let mut tablet_data = TabletData {
                    status,
                    x: report.x,
                    y: report.y,
                    pressure: report.pressure,
                    tilt_x: 0,
                    tilt_y: 0,
                    buttons: report.buttons,
                    eraser: report.eraser,
                    hover_distance: 0,
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
