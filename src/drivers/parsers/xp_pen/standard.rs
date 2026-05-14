use crate::drivers::TabletData;

#[must_use]
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
            let x = (u16::from(*x_high) << 8) | u16::from(*x_low);
            let y = (u16::from(*y_high) << 8) | u16::from(*y_low);
            let pressure = (u16::from(*p_high) << 8) | u16::from(*p_low);

            let (tilt_x, tilt_y) = match rest {
                [tx, ty, ..] => (tx.cast_signed(), ty.cast_signed()),
                [tx, ..] => (tx.cast_signed(), 0),
                _ => (0, 0),
            };

            let buttons = (*b1 >> 1) & 0x03;
            let eraser = (*b1 & 0x08) != 0;

            let raw = data
                .iter()
                .take(14)
                .map(|b| format!("{b:02X}"))
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
