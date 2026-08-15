//! Linux implementation of the HID owner lock via `flock(2)` on a file under
//! `XDG_RUNTIME_DIR`, tried non-blockingly. The kernel releases the lock
//! automatically when the holding process's file descriptors are closed —
//! including on crash or `SIGKILL` — so there is never a stale lock file to
//! clean up manually.

use nix::fcntl::{Flock, FlockArg};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

const LOCK_FILE_NAME: &str = "ntd_hid_owner.lock";

pub struct OwnerHandle(Flock<File>);

fn lock_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(dir).join(LOCK_FILE_NAME)
}

/// Tries to become the HID owner by opening (creating if needed) the lock
/// file and attempting a non-blocking exclusive `flock`.
pub fn try_acquire() -> Option<OwnerHandle> {
    let path = lock_path();
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)
        .ok()?;

    match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
        Ok(flock) => Some(OwnerHandle(flock)),
        Err((_file, _errno)) => None,
    }
}
