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

            let status = match b1 {
                0xA0 => crate::drivers::TabletStatus::Hover,
                0xA1 => crate::drivers::TabletStatus::Contact,
                0xC0 | 0x00 => crate::drivers::TabletStatus::OutOfRange,
                _ if (*b1 & 0x80) != 0 => crate::drivers::TabletStatus::OutOfRange,
                _ => crate::drivers::TabletStatus::Active,
            };

            let is_connected = status != crate::drivers::TabletStatus::OutOfRange;

            let mut tablet_data = TabletData {
                status,
                x,
                y,
                pressure,
                tilt_x,
                tilt_y,
                buttons,
                eraser,
                hover_distance: 0,
                is_connected,
                ..Default::default()
            };
            tablet_data.set_raw(data);
            Some(tablet_data)
        }
        _ => None,
    }
}
