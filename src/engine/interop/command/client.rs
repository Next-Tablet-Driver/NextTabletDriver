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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]
mod tests {
    use super::super::REQUEST_SIZE;
    use super::*;
    use interprocess::local_socket::ListenerOptions;

    /// Binds the command socket like an owner would and answers one request with `reply`
    /// (or hangs up without answering).
    fn fake_owner(reply: Option<&'static [u8]>) -> std::thread::JoinHandle<()> {
        let listener = ListenerOptions::new()
            .name(socket_name().unwrap())
            .create_sync()
            .unwrap();
        std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap();
            let mut request = [0u8; REQUEST_SIZE];
            stream.read_exact(&mut request).unwrap();
            if let Some(bytes) = reply {
                stream.write_all(bytes).unwrap();
            }
        })
    }

    #[test]
    fn without_an_owner_the_command_cannot_be_sent() {
        assert!(send_command(Request::Ping).is_err());
    }

    #[test]
    fn an_answer_that_is_not_a_response_is_malformed() {
        let owner = fake_owner(Some(&[7]));
        let error = send_command(Request::Ping).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        owner.join().unwrap();
    }

    #[test]
    fn an_owner_that_hangs_up_without_answering_is_an_error() {
        let owner = fake_owner(None);
        assert!(send_command(Request::Ping).is_err());
        owner.join().unwrap();
    }

    #[test]
    fn an_ok_answer_is_decoded() {
        let owner = fake_owner(Some(&[0]));
        assert_eq!(send_command(Request::Ping).unwrap(), Response::Ok);
        owner.join().unwrap();
    }

    #[test]
    fn a_rejection_is_decoded() {
        let owner = fake_owner(Some(&[1]));
        assert_eq!(send_command(Request::Ping).unwrap(), Response::Rejected);
        owner.join().unwrap();
    }
}
