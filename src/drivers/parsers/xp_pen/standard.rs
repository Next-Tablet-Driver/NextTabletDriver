use crate::drivers::TabletData;

pub fn parse(data: &[u8]) -> Option<TabletData> {
    match data {
        [
            _,
            b1,
            x_low,
            x_high,
            y_low,
            y_high,
            p_low,
            p_high,
            rest @ ..,
        ] => {
            let x = ((*x_high as u16) << 8) | (*x_low as u16);
            let y = ((*y_high as u16) << 8) | (*y_low as u16);
            let pressure = ((*p_high as u16) << 8) | (*p_low as u16);

            let (tilt_x, tilt_y) = match rest {
                [tx, ty, ..] => (*tx as i8, *ty as i8),
                [tx, ..] => (*tx as i8, 0),
                _ => (0, 0),
            };

            let buttons = (*b1 >> 1) & 0x03;
            let eraser = (*b1 & 0x08) != 0;

            let raw = data
                .iter()
                .take(14)
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");

            let status = match b1 {
                0xA0 => "Hover",
                0xA1 => "Contact",
                0xC0 | 0x00 => "Out of Range",
                _ if (*b1 & 0x80) != 0 => "Out of Range",
                _ => "Active",
            }
            .to_string();

            let is_connected = status != "Out of Range";

            Some(TabletData {
                status,
                x,
                y,
                pressure,
                tilt_x,
                tilt_y,
                buttons,
                eraser,
                hover_distance: 0,
                raw_data: raw,
                is_connected,
                ..Default::default()
            })
        }
        _ => None,
    }
}
