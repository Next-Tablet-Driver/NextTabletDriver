use crate::drivers::TabletData;
use crate::drivers::parsers::ReportParser;

fn parse_veikk_aux(data: &[u8], raw: String, offset: usize) -> Option<TabletData> {
    let buttons = data.get(offset).copied().unwrap_or(0);
    Some(TabletData {
        status: "Aux".to_string(),
        buttons,
        raw_data: raw,
        is_connected: true,
        ..Default::default()
    })
}

fn parse_veikk_tablet(data: &[u8], raw: String, has_tilt: bool) -> Option<TabletData> {
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
            let x = (*x_lo as u32) | ((*x_mid as u32) << 8) | ((*x_hi as u32) << 16);
            let y = (*y_lo as u32) | ((*y_mid as u32) << 8) | ((*y_hi as u32) << 16);
            let pressure = (*p_lo as u16) | ((*p_high as u16) << 8);
            let buttons = (*b2 >> 1) & 0x03;

            let (tilt_x, tilt_y) = if has_tilt {
                match rest {
                    [tx, ty, ..] => (*tx as i8, *ty as i8),
                    [tx, ..] => (*tx as i8, 0),
                    _ => (0, 0),
                }
            } else {
                (0, 0)
            };

            let status = if (*b2 & 0x20) != 0 {
                if pressure > 0 { "Contact" } else { "Hover" }
            } else {
                "Out of Range"
            };

            Some(TabletData {
                status: status.to_string(),
                x: x as u16,
                y: y as u16,
                pressure,
                tilt_x,
                tilt_y,
                buttons,
                raw_data: raw,
                is_connected: true,
                ..Default::default()
            })
        }
        _ => None,
    }
}

pub struct VeikkParser;

impl ReportParser for VeikkParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0x43, ..] => None, // Touchpad ignore
            [_, _, b2, ..] if (*b2 & 0x20) != 0 => parse_veikk_tablet(data, raw, false),
            [_, _, 0x01, ..] => parse_veikk_aux(data, raw, 4),
            _ => None,
        }
    }
}

pub struct VeikkV1Parser;

impl ReportParser for VeikkV1Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [0x03, ..] => parse_veikk_aux(data, raw, 1),
            [_, 0x41, 0xC0, ..] => None, // Out of Range
            [_, 0x41, ..] => parse_veikk_tablet(data, raw, false),
            _ => None,
        }
    }
}

pub struct VeikkA15Parser;

impl ReportParser for VeikkA15Parser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0x43, ..] => None, // Touchpad ignore
            [_, _, b2, ..] if (*b2 & 0x20) != 0 => parse_veikk_tablet(data, raw, false),
            [_, _, 0x01, ..] => parse_veikk_aux(data, raw, 4),
            _ => None,
        }
    }
}

pub struct VeikkTiltParser;

impl ReportParser for VeikkTiltParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let raw = data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        match data {
            [_, 0x41, 0xC0, ..] => None,
            [_, 0x41, ..] => parse_veikk_tablet(data, raw, true),
            [_, 0x42, ..] => parse_veikk_aux(data, raw, 4),
            [_, 0x43, ..] => None, // Touchpad ignore
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
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        Ok(())
    }
}
