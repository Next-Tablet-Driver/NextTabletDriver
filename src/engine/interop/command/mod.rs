//! # Command Channel
//!
//! Small fixed-size request/response protocol letting a non-owner process
//! forward config writes ([`Request::SetMode`], [`Request::SetActiveArea`])
//! to the current HID owner, the only process allowed to mutate the shared
//! tablet config, since it's the one actually driving the pipeline. The
//! owner listens on a well-known local socket ([`CommandListener`]); readers
//! connect once per command via [`send_command`].
//!
//! - [`protocol`] is the wire format: fixed-size encode/decode and the handler trait.
//! - [`client`] is the reader side (`send_command`).
//! - [`listener`] is the owner side (`CommandListener`).

mod client;
mod listener;
mod protocol;

pub use client::send_command;
pub use listener::CommandListener;
pub use protocol::{CommandHandler, Request, Response};

use interprocess::local_socket::{GenericFilePath, GenericNamespaced, Name, prelude::*};
use std::io;
use std::time::Duration;

const SOCKET_NAME: &str = "NextTabletDriver_Cmd_v1";

/// How often the listener thread wakes up to check for a shutdown request
/// while no client is connecting.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(20);

const REQUEST_SIZE: usize = 24;
const RESPONSE_SIZE: usize = 1;

fn socket_name() -> io::Result<Name<'static>> {
    socket_name_for(GenericNamespaced::is_supported())
}

fn socket_name_for(namespaced: bool) -> io::Result<Name<'static>> {
    if namespaced {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::core::config::models::{ActiveArea, DriverMode};
    use std::sync::{Arc, Mutex};

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

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;

        use interprocess::local_socket::Stream;
        use std::io::{Read, Write};

        fn exchange(bytes: &[u8]) -> Option<u8> {
            let mut stream = Stream::connect(socket_name().unwrap()).ok()?;
            stream.write_all(bytes).ok()?;
            let mut reply = [0u8; RESPONSE_SIZE];
            stream.read_exact(&mut reply).ok()?;
            Some(reply[0])
        }

        #[test]
        fn the_filesystem_fallback_lives_in_the_runtime_directory() {
            let path = runtime_socket_path();
            assert_eq!(path.file_name().unwrap(), "ntd_cmd_v1.sock");
        }

        #[test]
        fn the_namespaced_flavour_is_always_available_and_the_file_one_only_on_unix() {
            assert!(socket_name_for(true).is_ok());
            // Windows has no filesystem sockets: the fallback is refused there.
            assert_eq!(socket_name_for(false).is_ok(), cfg!(unix));
        }

        #[test]
        fn the_listener_survives_truncated_and_undecodable_requests() {
            let handler = Arc::new(RecordingHandler::default());
            let _listener = CommandListener::spawn(Arc::clone(&handler) as Arc<dyn CommandHandler>)
                .expect("listener should bind the command socket");

            // A client that hangs up halfway through a request gets no answer and breaks nothing.
            {
                let mut stream = Stream::connect(socket_name().unwrap()).unwrap();
                stream.write_all(&[1, 2, 3]).unwrap();
            }
            assert_eq!(send_command(Request::Ping).unwrap(), Response::Ok);

            // A full-size request that is not a command is answered with a rejection.
            let mut garbage = [0u8; REQUEST_SIZE];
            garbage[0] = 9;
            assert_eq!(exchange(&garbage), Some(Response::Rejected.encode()[0]));

            assert!(handler.modes.lock().unwrap().is_empty());
            assert!(handler.areas.lock().unwrap().is_empty());
        }
    }
}
