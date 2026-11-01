//! Thread priority through `SetThreadPriority`.
#![allow(unsafe_code, reason = "Win32 thread calls through windows-sys")]

use crate::PlatformError;
use windows_sys::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
};

/// Sets the calling thread to `TIME_CRITICAL`.
pub(crate) fn raise_current_thread_priority() -> Result<(), PlatformError> {
    // SAFETY: Retrieving the pseudo-handle of the current thread.
    let thread = unsafe { GetCurrentThread() };
    // SAFETY: Increasing thread priority to TIME_CRITICAL for low-latency USB polling.
    if unsafe { SetThreadPriority(thread, THREAD_PRIORITY_TIME_CRITICAL) } == 0 {
        Err(PlatformError::Os(
            "SetThreadPriority(TIME_CRITICAL) failed".to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thread_can_raise_its_own_priority() {
        // On a thread of its own, so that the test runner's threads keep their priority.
        let outcome = std::thread::spawn(raise_current_thread_priority).join();
        assert!(matches!(outcome, Ok(Ok(()))));
    }
}
