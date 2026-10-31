//! The protocol between the processes, without any transport.
//!
//! What goes over the wire and into the shared segment: the snapshot a reader gets
//! ([`SdkPublicState`]), the commands a reader sends ([`Request`], [`Response`]) and the handler
//! the owner implements. Nothing here opens a socket, maps memory or takes a lock, so it is
//! tested and fuzzed on its own; the transports are [`crate::shm`], [`crate::command`] and
//! [`crate::lock`].

use ntd_core::config::models::{ActiveArea, DriverMode};

/// Size in bytes of an encoded [`Request`].
pub const REQUEST_SIZE: usize = 24;

/// Size in bytes of an encoded [`Response`].
pub const RESPONSE_SIZE: usize = 1;

/// The name of the shared memory segment on Windows, in the `Local\` namespace of the session.
pub const SHM_NAME_WINDOWS: &str = "Local\\NextTabletDriver_State_v1";

/// The name of the shared memory segment on the POSIX systems. macOS limits the name of a
/// `shm_open` object to 31 characters, so it stays short.
pub const SHM_NAME_POSIX: &str = "/ntd_state_v1";

/// The longest name of a shared memory segment that every supported system accepts.
pub const SHM_NAME_MAX_LEN: usize = 31;

/// Bumped whenever [`SdkPublicState`]'s layout changes. A reader whose
/// compiled-in version doesn't match the segment's refuses to interpret the
/// payload rather than risk misreading a field.
pub const SDK_ABI_VERSION: u32 = 1;

/// Fixed capacity of [`SdkPublicState::device_name`], in bytes (UTF-8,
/// truncated, not necessarily NUL-terminated).
pub const DEVICE_NAME_CAPACITY: usize = 64;

/// Snapshot of live tablet state and the config it was produced under.
///
/// Published by the HID owner for every reader to consume. Mirrors the
/// fields the SDK's public `NtdState` FFI struct exposes; this struct *is*
/// that layout's single source of truth, shared across the process boundary.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdkPublicState {
    /// A pen is detected.
    pub is_connected: bool,
    /// Raw `TabletStatus` discriminant.
    pub status: u8,
    /// Horizontal position inside the active area, from 0 to 1.
    pub u: f32,
    /// Vertical position inside the active area, from 0 to 1.
    pub v: f32,
    /// Horizontal position on screen in pixels, or movement since the last packet in relative mode.
    pub screen_x: f32,
    /// Vertical position on screen in pixels, or movement since the last packet in relative mode.
    pub screen_y: f32,
    /// Pressure, from 0 to 8191.
    pub pressure: i32,
    /// Horizontal tilt, in degrees.
    pub tilt_x: i32,
    /// Vertical tilt, in degrees.
    pub tilt_y: i32,
    /// Bits of the pen buttons that are pressed.
    pub buttons: u8,
    /// The tip counts as touching.
    pub is_down: bool,
    /// The eraser end of the pen is in use.
    pub eraser: bool,
    /// UTF-8 device name, truncated to fit; only the first `device_name_len`
    /// bytes are meaningful.
    pub device_name: [u8; DEVICE_NAME_CAPACITY],
    /// Number of valid bytes in `device_name`.
    pub device_name_len: u32,
    /// USB vendor ID of the tablet.
    pub vid: u16,
    /// USB product ID of the tablet.
    pub pid: u16,
    /// Raw `DriverMode` discriminant.
    pub mode: u8,
    /// Left edge of the active area, in millimetres.
    pub active_area_x: f32,
    /// Top edge of the active area, in millimetres.
    pub active_area_y: f32,
    /// Width of the active area, in millimetres.
    pub active_area_w: f32,
    /// Height of the active area, in millimetres.
    pub active_area_h: f32,
    /// Rotation of the active area, in degrees.
    pub active_area_rotation: f32,
    /// Incremented by the owner every time the config changes, so readers
    /// (and command-channel callers) can tell config writes apart from noise.
    pub config_version: u32,
}

impl Default for SdkPublicState {
    fn default() -> Self {
        Self {
            is_connected: false,
            status: 0,
            u: 0.0,
            v: 0.0,
            screen_x: 0.0,
            screen_y: 0.0,
            pressure: 0,
            tilt_x: 0,
            tilt_y: 0,
            buttons: 0,
            is_down: false,
            eraser: false,
            device_name: [0u8; DEVICE_NAME_CAPACITY],
            device_name_len: 0,
            vid: 0,
            pid: 0,
            mode: 0,
            active_area_x: 0.0,
            active_area_y: 0.0,
            active_area_w: 0.0,
            active_area_h: 0.0,
            active_area_rotation: 0.0,
            config_version: 0,
        }
    }
}

/// A config write a reader wants the current HID owner to apply on its
/// behalf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Request {
    /// Liveness check: does nothing but confirms an owner is listening.
    Ping,
    /// Switches the driver mode.
    SetMode(DriverMode),
    /// Replaces the active area.
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
    /// The bytes that go over the socket: a tag, the mode, and the active area.
    #[must_use]
    pub const fn encode(self) -> [u8; REQUEST_SIZE] {
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

    /// The request that `buf` encodes, or `None` for an unknown tag or mode, or for an active
    /// area that is not made of finite numbers (a peer that sends NaN would feed it to the
    /// pipeline).
    #[must_use]
    pub fn decode(buf: [u8; REQUEST_SIZE]) -> Option<Self> {
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
            2 => {
                let area = ActiveArea {
                    x: f32::from_le_bytes([x0, x1, x2, x3]),
                    y: f32::from_le_bytes([y0, y1, y2, y3]),
                    w: f32::from_le_bytes([w0, w1, w2, w3]),
                    h: f32::from_le_bytes([h0, h1, h2, h3]),
                    rotation: f32::from_le_bytes([r0, r1, r2, r3]),
                };
                [area.x, area.y, area.w, area.h, area.rotation]
                    .iter()
                    .all(|v| v.is_finite())
                    .then_some(Self::SetActiveArea(area))
            }
            _ => None,
        }
    }
}

/// The owner's reply to a [`Request`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    /// The owner applied the request.
    Ok,
    /// The owner received the request but couldn't decode or apply it.
    Rejected,
}

impl Response {
    /// The byte that goes back over the socket.
    #[must_use]
    pub const fn encode(self) -> [u8; RESPONSE_SIZE] {
        match self {
            Self::Ok => [0],
            Self::Rejected => [1],
        }
    }

    /// The response that `buf` encodes, or `None` for an unknown tag.
    #[must_use]
    pub const fn decode(buf: [u8; RESPONSE_SIZE]) -> Option<Self> {
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
/// `ntd_state` itself. The desktop app and the SDK's embedded engine
/// loop each provide their own implementation that applies the write through
/// the exact same validation/config path a local caller would use.
pub trait CommandHandler: Send + Sync {
    /// Applies a mode change that a reader requested.
    fn set_mode(&self, mode: DriverMode);
    /// Applies an active area change that a reader requested.
    fn set_active_area(&self, area: ActiveArea);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_memory_names_fit_every_system() {
        // macOS refuses a `shm_open` name longer than 31 characters.
        assert!(SHM_NAME_POSIX.len() <= SHM_NAME_MAX_LEN, "{SHM_NAME_POSIX}");
        assert!(
            SHM_NAME_WINDOWS.len() <= SHM_NAME_MAX_LEN,
            "{SHM_NAME_WINDOWS}"
        );
        assert!(SHM_NAME_POSIX.starts_with('/'));
    }

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

    fn area() -> ActiveArea {
        ActiveArea {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
            rotation: 5.0,
        }
    }

    #[test]
    fn every_request_survives_an_encode_decode_round_trip() {
        for request in [
            Request::Ping,
            Request::SetMode(DriverMode::Absolute),
            Request::SetMode(DriverMode::Relative),
            Request::SetActiveArea(area()),
        ] {
            assert_eq!(Request::decode(request.encode()), Some(request));
        }
    }

    /// Found by the fuzzer: an area with NaN decoded, and a request must be equal to itself.
    #[test]
    fn an_active_area_that_is_not_finite_is_not_a_request() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut buf = Request::SetActiveArea(area()).encode();
            if let Some(slot) = buf.get_mut(4..8) {
                slot.copy_from_slice(&bad.to_le_bytes());
            }
            assert_eq!(Request::decode(buf), None, "{bad}");
        }
    }

    #[test]
    fn an_unknown_tag_is_not_a_request() {
        let mut buf = Request::Ping.encode();
        buf[0] = 9;
        assert_eq!(Request::decode(buf), None);
    }

    #[test]
    fn an_unknown_mode_byte_is_not_a_mode_change() {
        let mut buf = Request::SetMode(DriverMode::Relative).encode();
        buf[1] = 7;
        assert_eq!(Request::decode(buf), None);
    }

    #[test]
    fn responses_round_trip_and_unknown_tags_are_rejected() {
        for response in [Response::Ok, Response::Rejected] {
            assert_eq!(Response::decode(response.encode()), Some(response));
        }
        assert_eq!(Response::decode([2]), None);
        assert_eq!(Response::decode([255]), None);
    }
}
