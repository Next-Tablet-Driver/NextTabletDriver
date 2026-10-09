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
            let x = u32::from((u16::from(*x_high) << 8) | u16::from(*x_low));
            let y = u32::from((u16::from(*y_high) << 8) | u16::from(*y_low));
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::drivers::TabletStatus;

    /// `[id, b1, x(2), y(2), pressure(2), tilt_x, tilt_y]`, little-endian.
    fn report(b1: u8, x: u16, y: u16, pressure: u16, tilt: [u8; 2]) -> [u8; 10] {
        let (x, y, p) = (x.to_le_bytes(), y.to_le_bytes(), pressure.to_le_bytes());
        [
            0x02, b1, x[0], x[1], y[0], y[1], p[0], p[1], tilt[0], tilt[1],
        ]
    }

    #[test]
    fn contact_decodes_position_pressure_and_signed_tilt() {
        let parsed = parse(&report(0xA1, 0x1234, 0x5678, 0x0FFF, [0xFE, 0x05])).unwrap();
        assert_eq!(parsed.status, TabletStatus::Contact);
        assert_eq!(
            (parsed.x, parsed.y, parsed.pressure),
            (0x1234, 0x5678, 0x0FFF)
        );
        assert_eq!((parsed.tilt_x, parsed.tilt_y), (-2, 5));
        assert!(parsed.is_connected);
        assert_eq!(parsed.raw_len, 10);
    }

    #[test]
    fn the_status_byte_selects_the_status() {
        let status = |b1| parse(&report(b1, 1, 1, 1, [0, 0])).unwrap().status;
        assert_eq!(status(0xA0), TabletStatus::Hover);
        assert_eq!(status(0xA1), TabletStatus::Contact);
        assert_eq!(status(0xC0), TabletStatus::OutOfRange);
        assert_eq!(status(0x00), TabletStatus::OutOfRange);
        // Any other byte with the high bit set means the pen left the surface...
        assert_eq!(status(0x81), TabletStatus::OutOfRange);
        // ...and one without it is an active tool report.
        assert_eq!(status(0x02), TabletStatus::Active);
    }

    #[test]
    fn out_of_range_reports_mark_the_tablet_as_disconnected() {
        assert!(!parse(&report(0xC0, 1, 1, 1, [0, 0])).unwrap().is_connected);
        assert!(parse(&report(0xA0, 1, 1, 1, [0, 0])).unwrap().is_connected);
    }

    #[test]
    fn buttons_and_eraser_come_from_the_status_byte() {
        let buttons = parse(&report(0x06, 1, 1, 1, [0, 0])).unwrap();
        assert_eq!(buttons.buttons, 0b11);
        assert!(!buttons.eraser);
        let eraser = parse(&report(0x08, 1, 1, 1, [0, 0])).unwrap();
        assert!(eraser.eraser);
        assert_eq!(eraser.buttons, 0);
    }

    #[test]
    fn tilt_is_optional() {
        let full = report(0xA1, 1, 1, 1, [3, 4]);
        let one_tilt = parse(&full[..9]).unwrap();
        assert_eq!((one_tilt.tilt_x, one_tilt.tilt_y), (3, 0));
        let none = parse(&full[..8]).unwrap();
        assert_eq!((none.tilt_x, none.tilt_y), (0, 0));
    }

    #[test]
    fn short_reports_are_rejected() {
        assert!(parse(&report(0xA1, 1, 1, 1, [0, 0])[..7]).is_none());
        assert!(parse(&[]).is_none());
    }
}
