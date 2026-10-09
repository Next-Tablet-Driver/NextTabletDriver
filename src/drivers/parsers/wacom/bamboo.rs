use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

pub struct BambooTabletReport {
    pub x: u32,
    pub y: u32,
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
                let x = u32::from(u16::from_le_bytes([*x_lo, *x_hi]));
                let y = u32::from(u16::from_le_bytes([*y_lo, *y_hi]));

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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    fn report(b1: u8, x: u16, y: u16, p_lo: u8, p_hi_aux: u8) -> [u8; 8] {
        let (x, y) = (x.to_le_bytes(), y.to_le_bytes());
        [0x02, b1, x[0], x[1], y[0], y[1], p_lo, p_hi_aux]
    }

    #[test]
    fn contact_needs_the_tip_flag_and_decodes_pressure() {
        let parsed = BambooParser
            .parse(&report(0x81, 0x0102, 0x0304, 0x20, 0x01))
            .unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!((parsed.x, parsed.y), (0x0102, 0x0304));
        assert_eq!(parsed.pressure, 0x120);
        assert!(parsed.is_connected);
    }

    #[test]
    fn proximity_without_tip_is_hover_and_nothing_is_out_of_range() {
        let hover = BambooParser.parse(&report(0x80, 1, 1, 0xFF, 0x03)).unwrap();
        assert_eq!(hover.status, TabletStatus::Hover);
        assert_eq!(hover.pressure, 0);

        let away = BambooParser.parse(&report(0x00, 1, 1, 0, 0)).unwrap();
        assert_eq!(away.status, TabletStatus::OutOfRange);
    }

    #[test]
    fn barrel_buttons_and_eraser() {
        let parsed = BambooParser
            .parse(&report(0x80 | 0x20 | 0x04, 1, 1, 0, 0))
            .unwrap();
        assert!(parsed.eraser);
        assert_eq!(parsed.buttons, 0b10);
    }

    #[test]
    fn tablet_report_exposes_the_four_aux_buttons() {
        let all = BambooTabletReport::new(&report(0x80, 0, 0, 0, 0x78)).unwrap();
        assert_eq!(all.aux_buttons, [true; 4]);
        let second = BambooTabletReport::new(&report(0x80, 0, 0, 0, 0x10)).unwrap();
        assert_eq!(second.aux_buttons, [false, true, false, false]);
    }

    #[test]
    fn rejects_other_report_ids_and_short_reports() {
        let mut data = report(0x81, 1, 1, 1, 0);
        data[0] = 0x10;
        assert!(BambooParser.parse(&data).is_none());
        assert!(BambooTabletReport::new(&data[..7]).is_none());
        assert!(BambooParser.parse(&[0x02, 0x81]).is_none());
    }
}
