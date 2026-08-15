//! # Command Channel
//!
//! Small fixed-size request/response protocol letting a non-owner process
//! forward config writes ([`Request::SetMode`], [`Request::SetActiveArea`])
//! to the current HID owner — the only process allowed to mutate the shared
//! tablet config, since it's the one actually driving the pipeline. The
//! owner listens on a well-known local socket ([`CommandListener`]); readers
//! connect once per command via [`send_command`].

use crate::core::config::models::{ActiveArea, DriverMode};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, Listener, ListenerNonblockingMode, ListenerOptions, Name,
    Stream, prelude::*,
};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

const SOCKET_NAME: &str = "NextTabletDriver_Cmd_v1";

/// How often the listener thread wakes up to check for a shutdown request
/// while no client is connecting.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(20);

fn socket_name() -> io::Result<Name<'static>> {
    if GenericNamespaced::is_supported() {
        SOCKET_NAME.to_ns_name::<GenericNamespaced>()
    } else {
        runtime_socket_path().to_fs_name::<GenericFilePath>()
    }
}

/// Filesystem fallback for platforms without a socket namespace, scoped
/// under the same runtime directory the HID owner lock file uses.
fn runtime_socket_path() -> std::path::PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(dir).join("ntd_cmd_v1.sock")
}

const REQUEST_SIZE: usize = 24;
const RESPONSE_SIZE: usize = 1;

/// A config write a reader wants the current HID owner to apply on its
/// behalf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Request {
    /// Liveness check — does nothing but confirms an owner is listening.
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
    const fn encode(self) -> [u8; REQUEST_SIZE] {
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

    fn decode(buf: [u8; REQUEST_SIZE]) -> Option<Self> {
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
    const fn encode(self) -> [u8; RESPONSE_SIZE] {
        match self {
            Self::Ok => [0],
            Self::Rejected => [1],
        }
    }

    const fn decode(buf: [u8; RESPONSE_SIZE]) -> Option<Self> {
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

/// Sends a single command to whichever process currently owns the HID device
/// and waits for its response.
///
/// # Errors
///
/// Returns `Err` if no owner is currently listening (e.g. between a
/// promotion and the new owner starting its listener) — callers should treat
/// that as "try again shortly," not as a hard failure.
pub fn send_command(request: Request) -> io::Result<Response> {
    let name = socket_name()?;
    let mut stream = Stream::connect(name)?;
    stream.write_all(&request.encode())?;
    let mut buf = [0u8; RESPONSE_SIZE];
    stream.read_exact(&mut buf)?;
    Response::decode(buf)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "malformed command response"))
}

fn handle_connection(mut stream: Stream, handler: &Arc<dyn CommandHandler>) {
    let mut buf = [0u8; REQUEST_SIZE];
    if stream.read_exact(&mut buf).is_err() {
        return;
    }
    let response = match Request::decode(buf) {
        Some(Request::Ping) => Response::Ok,
        Some(Request::SetMode(mode)) => {
            handler.set_mode(mode);
            Response::Ok
        }
        Some(Request::SetActiveArea(area)) => {
            handler.set_active_area(area);
            Response::Ok
        }
        None => Response::Rejected,
    };
    let _ = stream.write_all(&response.encode());
}

/// Owns the background thread that accepts and answers commands on behalf of
/// the current HID owner. Dropping it stops the listener.
pub struct CommandListener {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl CommandListener {
    /// Starts listening on the well-known command socket, dispatching every
    /// incoming request to `handler`. Only the current HID owner should call
    /// this — readers use [`send_command`] instead.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the well-known socket name is already bound by
    /// another listener (shouldn't happen in practice: only the HID owner,
    /// which is unique per [`super::lock`], ever spawns one).
    pub fn spawn(handler: Arc<dyn CommandHandler>) -> io::Result<Self> {
        let name = socket_name()?;
        let listener: Listener = ListenerOptions::new().name(name).create_sync()?;
        listener.set_nonblocking(ListenerNonblockingMode::Accept)?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_thread = Arc::clone(&shutdown);
        let handle = std::thread::spawn(move || {
            while !shutdown_for_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(stream) => handle_connection(stream, &handler),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(ACCEPT_POLL_INTERVAL);
                    }
                    Err(_) => std::thread::sleep(ACCEPT_POLL_INTERVAL),
                }
            }
        });

        Ok(Self {
            shutdown,
            handle: Some(handle),
        })
    }
}

impl Drop for CommandListener {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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

    #[derive(Default)]
    struct RecordingHandler {
        modes: Mutex<Vec<DriverMode>>,
        areas: Mutex<Vec<ActiveArea>>,
    }

    impl CommandHandler for RecordingHandler {
        fn set_mode(&self, mode: DriverMode) {
            self.modes
                .lock()
                .expect("mutex should not be poisoned")
                .push(mode);
        }

        fn set_active_area(&self, area: ActiveArea) {
            self.areas
                .lock()
                .expect("mutex should not be poisoned")
                .push(area);
        }
    }

    #[test]
    fn listener_dispatches_commands_to_handler() {
        let handler = Arc::new(RecordingHandler::default());
        let listener = CommandListener::spawn(Arc::clone(&handler) as Arc<dyn CommandHandler>)
            .expect("listener should bind the command socket");

        let area = ActiveArea {
            x: 10.0,
            y: 20.0,
            w: 30.0,
            h: 40.0,
            rotation: 90.0,
        };
        assert_eq!(
            send_command(Request::Ping).expect("ping should succeed"),
            Response::Ok
        );
        assert_eq!(
            send_command(Request::SetMode(DriverMode::Relative)).expect("set mode should succeed"),
            Response::Ok
        );
        assert_eq!(
            send_command(Request::SetActiveArea(area)).expect("set active area should succeed"),
            Response::Ok
        );

        assert_eq!(
            handler
                .modes
                .lock()
                .expect("mutex should not be poisoned")
                .as_slice(),
            [DriverMode::Relative]
        );
        assert_eq!(
            handler
                .areas
                .lock()
                .expect("mutex should not be poisoned")
                .as_slice(),
            [area]
        );

        drop(listener);
    }
}
