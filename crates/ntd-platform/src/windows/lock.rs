//! Windows implementation of the named locks via a named mutex (the HID owner is
//! `Local\NextTabletDriver_HidOwner`), tried non-blockingly.
#![allow(unsafe_code, reason = "Win32 named mutex calls through windows-sys")]

use crate::NamedLock;
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

/// The kernel object name of `lock`. These names are shared with the SDK of integrators: never
/// change them.
const fn mutex_name(lock: NamedLock) -> &'static str {
    match lock {
        NamedLock::HidOwner => "Local\\NextTabletDriver_HidOwner",
    }
}

/// Wraps the mutex handle. Dropping it releases and closes the handle, which
/// hands ownership back to the OS and makes the lock available to the next
/// process that tries, including automatically if this process crashes.
pub(crate) struct LockHandle(HANDLE);

// SAFETY: the wrapped HANDLE is a plain Win32 kernel object reference with no
// thread-affinity requirements; it's fine to move between threads as long as
// access stays exclusive (guaranteed here since `LockHandle` isn't `Clone`).
unsafe impl Send for LockHandle {}

impl Drop for LockHandle {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a valid mutex handle owned by this guard and not
        // yet closed; this is the only place it is released.
        unsafe {
            ReleaseMutex(self.0);
        }
        // SAFETY: `self.0` was just released above and is still a valid open
        // handle; closing it returns the OS handle table slot.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Tries to take `lock` by creating (or opening) its named mutex and attempting to acquire it
/// with a zero-millisecond wait.
pub(crate) fn try_acquire_lock(lock: NamedLock) -> Option<LockHandle> {
    try_acquire_named(mutex_name(lock))
}

fn try_acquire_named(name: &str) -> Option<LockHandle> {
    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

    // SAFETY: `wide_name` is a valid, NUL-terminated UTF-16 buffer kept alive
    // for the duration of the call; no security attributes or initial-owner
    // request are used, so no further invariants apply.
    let handle = unsafe { CreateMutexW(ptr::null(), 0, wide_name.as_ptr()) };
    if handle.is_null() {
        return None;
    }

    // SAFETY: `handle` was just created above and is a valid, open mutex
    // handle for the duration of this call.
    let wait_result = unsafe { WaitForSingleObject(handle, 0) };

    // WAIT_ABANDONED means the previous owner died while holding the mutex;
    // we still get ownership, which is exactly the self-healing promotion
    // behavior this lock is meant to provide.
    if wait_result == WAIT_OBJECT_0 || wait_result == WAIT_ABANDONED {
        Some(LockHandle(handle))
    } else {
        // SAFETY: `handle` is valid; the wait above did not grant ownership,
        // so closing it here is safe and required to avoid leaking the handle.
        unsafe {
            CloseHandle(handle);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_locks_are_the_ones_the_sdk_uses() {
        assert_eq!(
            mutex_name(NamedLock::HidOwner),
            "Local\\NextTabletDriver_HidOwner"
        );
    }

    #[test]
    fn a_name_the_os_refuses_gives_no_lock() {
        // Kernel object names cannot contain a backslash after the namespace prefix.
        assert!(try_acquire_named("Local\\ntd\\not\\valid").is_none());
    }

    #[test]
    fn the_lock_is_exclusive_across_threads_and_released_on_drop() {
        let name = format!("Local\\NtdTest_{}", std::process::id());
        let first = try_acquire_named(&name).expect("first acquisition");
        let contender = {
            let name = name.clone();
            std::thread::spawn(move || try_acquire_named(&name).is_some())
        };
        assert!(!contender.join().unwrap());
        drop(first);

        let again = std::thread::spawn(move || try_acquire_named(&name).is_some());
        assert!(again.join().unwrap());
    }
}
