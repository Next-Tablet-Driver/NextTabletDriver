//! Owner-side: accepting and dispatching commands on the well-known socket.

use super::{ACCEPT_POLL_INTERVAL, CommandHandler, REQUEST_SIZE, Request, Response, socket_name};

use interprocess::local_socket::{
    Listener, ListenerNonblockingMode, ListenerOptions, Stream, prelude::*,
};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

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
    /// this; readers use [`super::send_command`] instead.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the well-known socket name is already bound by
    /// another listener (shouldn't happen in practice: only the HID owner,
    /// which is unique per [`super::super::lock`], ever spawns one).
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
