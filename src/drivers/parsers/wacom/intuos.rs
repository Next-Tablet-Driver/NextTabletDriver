use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct IntuosTabletReport {
    pub x: u32,
    pub y: u32,
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
                x: u32::from(u16::from_le_bytes([*x_lo, *x_hi])),
                y: u32::from(u16::from_le_bytes([*y_lo, *y_hi])),
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
        match data {
            [0x02, b1, ..] if (*b1 & 0x40) != 0 => {
                let report = IntuosTabletReport::new(data)?;

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
                    hover_distance: report.hover_distance,
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

pub struct WacomDriverIntuosParser;

impl ReportParser for WacomDriverIntuosParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, rest @ ..] => IntuosParser.parse(rest),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    fn report(b1: u8, x: u16, y: u16, pressure: u16, hover: u8) -> [u8; 9] {
        let (x, y, p) = (x.to_le_bytes(), y.to_le_bytes(), pressure.to_le_bytes());
        [0x02, b1, x[0], x[1], y[0], y[1], p[0], p[1], hover]
    }

    #[test]
    fn contact_decodes_position_pressure_and_hover_distance() {
        let parsed = IntuosParser
            .parse(&report(0xC0, 0x1234, 0x0567, 0x0110, 7))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!(
            (parsed.x, parsed.y, parsed.pressure),
            (0x1234, 0x0567, 0x0110)
        );
        assert_eq!(parsed.hover_distance, 7);
        assert_eq!(parsed.buttons, 0);
        assert!(!parsed.eraser);
    }

    #[test]
    fn status_follows_pressure_then_proximity() {
        let hover = IntuosParser.parse(&report(0xC0, 1, 1, 0, 3)).unwrap();
        assert_eq!(hover.status, TabletStatus::Hover);
        let away = IntuosParser.parse(&report(0x40, 1, 1, 0, 0)).unwrap();
        assert_eq!(away.status, TabletStatus::OutOfRange);
    }

    #[test]
    fn eraser_flag_and_barrel_buttons() {
        let parsed = IntuosParser
            .parse(&report(0xC0 | 0x08 | 0x04, 1, 1, 1, 0))
            .unwrap();
        assert!(parsed.eraser);
        assert_eq!(parsed.buttons, 0b10);
    }

    #[test]
    fn only_tool_reports_are_parsed() {
        // Bit 0x40 marks a tool report; other report ids and shapes are not ours.
        assert!(IntuosParser.parse(&report(0x80, 1, 1, 1, 0)).is_none());
        let mut other_id = report(0xC0, 1, 1, 1, 0);
        other_id[0] = 0x03;
        assert!(IntuosParser.parse(&other_id).is_none());
        // A report one byte short of the full layout is rejected, not misread.
        assert!(IntuosParser.parse(&report(0xC0, 1, 1, 1, 0)[..8]).is_none());
        assert!(IntuosTabletReport::new(&[]).is_none());
    }

    #[test]
    fn the_driver_variant_skips_the_leading_byte() {
        let inner = report(0xC0, 40, 50, 60, 1);
        let mut framed = vec![0xAA];
        framed.extend_from_slice(&inner);
        let parsed = WacomDriverIntuosParser.parse(&framed).unwrap();
        assert_eq!((parsed.x, parsed.y, parsed.pressure), (40, 50, 60));
        assert!(WacomDriverIntuosParser.parse(&[]).is_none());
    }
}
