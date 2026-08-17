//! Windows implementation of the HID owner lock via a named mutex
//! (`Local\NextTabletDriver_HidOwner`), tried non-blockingly.

use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

const MUTEX_NAME: &str = "Local\\NextTabletDriver_HidOwner";

/// Wraps the mutex handle. Dropping it releases and closes the handle, which
/// hands ownership back to the OS and makes the lock available to the next
/// process that tries, including automatically if this process crashes.
pub struct OwnerHandle(HANDLE);

// SAFETY: the wrapped HANDLE is a plain Win32 kernel object reference with no
// thread-affinity requirements; it's fine to move between threads as long as
// access stays exclusive (guaranteed here since `OwnerHandle` isn't `Clone`).
unsafe impl Send for OwnerHandle {}

impl Drop for OwnerHandle {
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

/// Tries to become the HID owner by creating (or opening) the named mutex and
/// attempting to acquire it with a zero-millisecond wait.
pub fn try_acquire() -> Option<OwnerHandle> {
    let wide_name: Vec<u16> = MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

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
        Some(OwnerHandle(handle))
    } else {
        // SAFETY: `handle` is valid; the wait above did not grant ownership,
        // so closing it here is safe and required to avoid leaking the handle.
        unsafe {
            CloseHandle(handle);
        }
        None
    }
}
