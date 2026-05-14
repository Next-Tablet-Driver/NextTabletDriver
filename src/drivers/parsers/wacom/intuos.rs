use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct IntuosTabletReport {
    pub x: u16,
    pub y: u16,
    pub pressure: u16,
    pub eraser: bool,
    pub near_proximity: bool,
    pub buttons: u8,
    pub hover_distance: u8,
}

impl IntuosTabletReport {
    #[must_use] 
    pub fn new(report: &[u8]) -> Option<Self> {
        match report {
            [_, b1, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, h_dist, ..] => Some(Self {
                x: u16::from_le_bytes([*x_lo, *x_hi]),
                y: u16::from_le_bytes([*y_lo, *y_hi]),
                pressure: u16::from_le_bytes([*p_lo, *p_hi]),
                eraser: (*b1 & 0x08) != 0,
                near_proximity: (*b1 & 0x80) != 0,
                buttons: (*b1 >> 1) & 0x03,
                hover_distance: *h_dist,
            }),
            _ => None,
        }
    }
}

pub struct IntuosParser;

impl ReportParser for IntuosParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [0x02, b1, ..] if (*b1 & 0x40) != 0 => {
                let report = IntuosTabletReport::new(data)?;

                Some(TabletData {
                    status: if report.pressure > 0 {
                        "Contact".to_string()
                    } else if report.near_proximity {
                        "Hover".to_string()
                    } else {
                        "Out of Range".to_string()
                    },
                    x: report.x,
                    y: report.y,
                    pressure: report.pressure,
                    tilt_x: 0,
                    tilt_y: 0,
                    buttons: report.buttons,
                    eraser: report.eraser,
                    hover_distance: report.hover_distance,
                    raw_data: raw,
                    is_connected: true,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }
}

pub struct WacomDriverIntuosParser;

impl ReportParser for WacomDriverIntuosParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, rest @ ..] => IntuosParser.parse(rest),
            _ => None,
        }
    }
}
