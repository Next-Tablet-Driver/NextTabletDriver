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
}
