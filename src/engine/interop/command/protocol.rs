//! Wire format: fixed-size request/response encoding and the handler trait.

use crate::core::config::models::{ActiveArea, DriverMode};

use super::{REQUEST_SIZE, RESPONSE_SIZE};

/// A config write a reader wants the current HID owner to apply on its
/// behalf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Request {
    /// Liveness check: does nothing but confirms an owner is listening.
    Ping,
    SetMode(DriverMode),
    SetActiveArea(ActiveArea),
}

const fn mode_to_byte(mode: DriverMode) -> u8 {
    match mode {
        DriverMode::Absolute => 0,
        DriverMode::Relative => 1,
    }
}

const fn byte_to_mode(byte: u8) -> Option<DriverMode> {
    match byte {
        0 => Some(DriverMode::Absolute),
        1 => Some(DriverMode::Relative),
        _ => None,
    }
}

impl Request {
    pub(super) const fn encode(self) -> [u8; REQUEST_SIZE] {
        let (tag, mode_byte, area) = match self {
            Self::Ping => (
                0u8,
                0u8,
                ActiveArea {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                    rotation: 0.0,
                },
            ),
            Self::SetMode(mode) => (
                1u8,
                mode_to_byte(mode),
                ActiveArea {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                    rotation: 0.0,
                },
            ),
            Self::SetActiveArea(area) => (2u8, 0u8, area),
        };
        let [x0, x1, x2, x3] = area.x.to_le_bytes();
        let [y0, y1, y2, y3] = area.y.to_le_bytes();
        let [w0, w1, w2, w3] = area.w.to_le_bytes();
        let [h0, h1, h2, h3] = area.h.to_le_bytes();
        let [r0, r1, r2, r3] = area.rotation.to_le_bytes();
        [
            tag, mode_byte, 0, 0, x0, x1, x2, x3, y0, y1, y2, y3, w0, w1, w2, w3, h0, h1, h2, h3,
            r0, r1, r2, r3,
        ]
    }

    pub(super) fn decode(buf: [u8; REQUEST_SIZE]) -> Option<Self> {
        let [
            tag,
            mode_byte,
            _,
            _,
            x0,
            x1,
            x2,
            x3,
            y0,
            y1,
            y2,
            y3,
            w0,
            w1,
            w2,
            w3,
            h0,
            h1,
            h2,
            h3,
            r0,
            r1,
            r2,
            r3,
        ] = buf;
        match tag {
            0 => Some(Self::Ping),
            1 => byte_to_mode(mode_byte).map(Self::SetMode),
            2 => Some(Self::SetActiveArea(ActiveArea {
                x: f32::from_le_bytes([x0, x1, x2, x3]),
                y: f32::from_le_bytes([y0, y1, y2, y3]),
                w: f32::from_le_bytes([w0, w1, w2, w3]),
                h: f32::from_le_bytes([h0, h1, h2, h3]),
                rotation: f32::from_le_bytes([r0, r1, r2, r3]),
            })),
            _ => None,
        }
    }
}

/// The owner's reply to a [`Request`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    Ok,
    /// The owner received the request but couldn't decode or apply it.
    Rejected,
}

impl Response {
    pub(super) const fn encode(self) -> [u8; RESPONSE_SIZE] {
        match self {
            Self::Ok => [0],
            Self::Rejected => [1],
        }
    }

    pub(super) const fn decode(buf: [u8; RESPONSE_SIZE]) -> Option<Self> {
        let [tag] = buf;
        match tag {
            0 => Some(Self::Ok),
            1 => Some(Self::Rejected),
            _ => None,
        }
    }
}

/// Implemented by whatever owns the real `SharedState` on the HID-owner side.
///
/// Kept as a trait so `engine::interop::command` doesn't need to depend on
/// `engine::state` itself. The desktop app and the SDK's embedded engine
/// loop each provide their own implementation that applies the write through
/// the exact same validation/config path a local caller would use.
pub trait CommandHandler: Send + Sync {
    fn set_mode(&self, mode: DriverMode);
    fn set_active_area(&self, area: ActiveArea);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_through_encode_decode() {
        let requests = [
            Request::Ping,
            Request::SetMode(DriverMode::Relative),
            Request::SetActiveArea(ActiveArea {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
                rotation: 5.0,
            }),
        ];
        for request in requests {
            assert_eq!(Request::decode(request.encode()), Some(request));
        }
    }

    #[test]
    fn response_round_trips_through_encode_decode() {
        for response in [Response::Ok, Response::Rejected] {
            assert_eq!(Response::decode(response.encode()), Some(response));
        }
    }
}
