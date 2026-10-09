//! Reader-side: sending a single command to the current HID owner.

use super::{RESPONSE_SIZE, Request, Response, socket_name};

use interprocess::local_socket::{Stream, prelude::*};
use std::io::{self, Read, Write};

/// Sends a single command to whichever process currently owns the HID device
/// and waits for its response.
///
/// # Errors
///
/// Returns `Err` if no owner is currently listening (e.g. between a
/// promotion and the new owner starting its listener). Callers should treat
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
