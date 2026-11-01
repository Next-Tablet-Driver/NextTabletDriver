//! Linux thread priority through `nice(2)`.
#![allow(unsafe_code, reason = "a libc call to change the niceness")]

use crate::PlatformError;

/// Lowers the niceness of the calling thread to -11. It needs `CAP_SYS_NICE` or root.
pub(crate) fn raise_current_thread_priority() -> Result<(), PlatformError> {
    // SAFETY: `libc::nice` only changes the niceness of the calling thread. -1 is both a possible
    // result and the error value, but -11 can never give -1, so -1 is an error.
    let result = unsafe { libc::nice(-11) };
    if result == -1 {
        Err(PlatformError::PermissionDenied)
    } else {
        Ok(())
    }
}
