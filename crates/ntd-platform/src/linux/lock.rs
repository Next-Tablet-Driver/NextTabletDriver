//! Linux implementation of the named locks via `flock(2)` on a file under `XDG_RUNTIME_DIR`,
//! tried non-blockingly. The kernel releases the lock automatically when the holding process's
//! file descriptors are closed, including on crash or `SIGKILL`, so there is never a stale lock
//! file to clean up manually.

use crate::NamedLock;
use nix::fcntl::{Flock, FlockArg};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// The file name of `lock`. These names are shared with the SDK of integrators: never change
/// them.
const fn file_name(lock: NamedLock) -> &'static str {
    match lock {
        NamedLock::HidOwner => "ntd_hid_owner.lock",
    }
}

// The `Flock` is held only for its `Drop` side effect (releasing the lock
// when the handle is dropped); it is never read directly.
#[allow(dead_code, reason = "held only for its `Drop` side effect")]
pub(crate) struct LockHandle(Flock<File>);

fn lock_path(lock: NamedLock) -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(dir).join(file_name(lock))
}

/// Tries to take `lock` by opening (creating if needed) its file and attempting a non-blocking
/// exclusive `flock`.
pub(crate) fn try_acquire_lock(lock: NamedLock) -> Option<LockHandle> {
    try_acquire_at(&lock_path(lock))
}

fn try_acquire_at(path: &Path) -> Option<LockHandle> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .ok()?;

    match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
        Ok(flock) => Some(LockHandle(flock)),
        Err((_file, _errno)) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_locks_are_the_ones_the_sdk_uses() {
        assert_eq!(file_name(NamedLock::HidOwner), "ntd_hid_owner.lock");
    }

    #[test]
    fn the_lock_is_exclusive_and_released_on_drop() {
        let path = std::env::temp_dir().join(format!("ntd_platform_lock_{}", std::process::id()));
        let first = try_acquire_at(&path).expect("first acquisition");
        // A second open file description conflicts, whichever thread asks.
        assert!(try_acquire_at(&path).is_none());
        drop(first);
        assert!(try_acquire_at(&path).is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_path_that_cannot_be_opened_gives_no_lock() {
        assert!(try_acquire_at(Path::new("/nonexistent-dir/ntd.lock")).is_none());
    }
}
