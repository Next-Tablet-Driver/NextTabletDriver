use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

fn parse_veikk_aux(data: &[u8], offset: usize) -> TabletData {
    let buttons = data.get(offset).copied().unwrap_or(0);
    let mut tablet_data = TabletData {
        status: crate::drivers::TabletStatus::Aux,
        buttons,
        is_connected: true,
        ..Default::default()
    };
    tablet_data.set_raw(data);
    tablet_data
}

fn parse_veikk_tablet(data: &[u8], has_tilt: bool) -> Option<TabletData> {
    match data {
        [
            _,
            _,
            b2,
            x_lo,
            x_mid,
            x_hi,
            y_lo,
            y_mid,
            y_hi,
            p_lo,
            p_high,
            rest @ ..,
        ] => {
            let x = u32::from(*x_lo) | (u32::from(*x_mid) << 8) | (u32::from(*x_hi) << 16);
            let y = u32::from(*y_lo) | (u32::from(*y_mid) << 8) | (u32::from(*y_hi) << 16);
            let pressure = u16::from(*p_lo) | (u16::from(*p_high) << 8);
            let buttons = (*b2 >> 1) & 0x03;

            let (tilt_x, tilt_y) = if has_tilt {
                match rest {
                    [tx, ty, ..] => (tx.cast_signed(), ty.cast_signed()),
                    [tx, ..] => (tx.cast_signed(), 0),
                    _ => (0, 0),
                }
            } else {
                (0, 0)
            };

            let status = if (*b2 & 0x20) != 0 {
                if pressure > 0 {
                    crate::drivers::TabletStatus::Contact
                } else {
                    crate::drivers::TabletStatus::Hover
                }
            } else {
                crate::drivers::TabletStatus::OutOfRange
            };

            let mut tablet_data = TabletData {
                status,
                x: x as u16,
                y: y as u16,
                pressure,
                tilt_x,
                tilt_y,
                buttons,
                is_connected: true,
                ..Default::default()
            };
            tablet_data.set_raw(data);
            Some(tablet_data)
        }
        _ => None,
    }
}

pub struct VeikkParser;

impl ReportParser for VeikkParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0x43, ..] => None, // Touchpad ignore
            [_, _, b2, ..] if (*b2 & 0x20) != 0 => parse_veikk_tablet(data, false),
            [_, _, 0x01, ..] => Some(parse_veikk_aux(data, 4)),
            _ => None,
        }
    }
}

pub struct VeikkV1Parser;

impl ReportParser for VeikkV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [0x03, ..] => Some(parse_veikk_aux(data, 1)),
            [_, 0x41, 0xC0, ..] => None, // Out of Range
            [_, 0x41, ..] => parse_veikk_tablet(data, false),
            _ => None,
        }
    }
}

pub struct VeikkA15Parser;

impl ReportParser for VeikkA15Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0x43, ..] => None, // Touchpad ignore
            [_, _, b2, ..] if (*b2 & 0x20) != 0 => parse_veikk_tablet(data, false),
            [_, _, 0x01, ..] => Some(parse_veikk_aux(data, 4)),
            _ => None,
        }
    }
}

pub struct VeikkTiltParser;

impl ReportParser for VeikkTiltParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        match data {
            [_, 0x41, 0xC0, ..] | [_, 0x43, ..] => None, // Out of Range or Touchpad ignore
            [_, 0x41, ..] => parse_veikk_tablet(data, true),
            [_, 0x42, ..] => Some(parse_veikk_aux(data, 4)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_veikk_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = VeikkParser;
        let data: [u8; 11] = [
            0x01, 0x02, 0x22, 0x02, 0x01, 0x00, 0x04, 0x03, 0x00, 0x01, 0x00,
        ];
        let report = parser
            .parse(&data)
            .ok_or("Veikk parser failed to parse tablet packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        Ok(())
    }
}
