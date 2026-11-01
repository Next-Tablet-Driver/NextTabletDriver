//! The system timer resolution through `ntdll`.
#![allow(unsafe_code, reason = "ntdll and winmm calls through windows-sys")]

use crate::PlatformError;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

/// `STATUS_TIMER_RESOLUTION_NOT_SET`: there is no request of ours left to cancel.
const STATUS_TIMER_RESOLUTION_NOT_SET: u32 = 0xC000_0245;

/// Adjusts the Windows system timer resolution to minimize input latency.
///
/// # Technical Details
/// By default, Windows uses a timer interval of ~15.6ms. For a high-performance
/// tablet driver, this can lead to "aliasing" or "jitter" where tablet reports
/// (often 1000Hz+) are processed in inconsistent batches.
///
/// This function calls the undocumented `NtSetTimerResolution` in `ntdll.dll`
/// to force a **0.5ms** (5000 units of 100ns) resolution, the maximum
/// precision supported by the Windows kernel.
pub(crate) fn set_high_resolution_timer(enable: bool) -> Result<(), PlatformError> {
    // SAFETY: `GetModuleHandleA` is called with a valid static C string literal.
    let ntdll = unsafe { GetModuleHandleA(c"ntdll.dll".as_ptr().cast::<u8>()) };
    if ntdll.is_null() {
        return Err(PlatformError::Os(
            "Failed to get the ntdll handle for the timer resolution".to_string(),
        ));
    }

    // SAFETY: `GetProcAddress` is called with a valid static C string function name on a verified
    // ntdll handle.
    let addr_set = unsafe { GetProcAddress(ntdll, c"NtSetTimerResolution".as_ptr().cast::<u8>()) };
    // SAFETY: Same as above; the name is a valid static C string and `ntdll` is a valid handle.
    let addr_query =
        unsafe { GetProcAddress(ntdll, c"NtQueryTimerResolution".as_ptr().cast::<u8>()) };

    let (Some(addr_set), Some(addr_query)) = (addr_set, addr_query) else {
        return Err(PlatformError::Os(
            "Could not find the timer resolution functions in ntdll.dll".to_string(),
        ));
    };

    // SAFETY: `NtSetTimerResolution` has this exact signature (see the documented prototype).
    let nt_set: unsafe extern "system" fn(u32, u8, *mut u32) -> i32 =
        unsafe { std::mem::transmute(addr_set) };
    // SAFETY: `NtQueryTimerResolution` has this exact signature (see the documented prototype).
    let nt_query: unsafe extern "system" fn(*mut u32, *mut u32, *mut u32) -> i32 =
        unsafe { std::mem::transmute(addr_query) };

    let mut min = 0;
    let mut max = 0;
    let mut cur = 0;

    // SAFETY: All three pointers refer to live local `u32`s.
    let _ = unsafe { nt_query(&raw mut min, &raw mut max, &raw mut cur) };

    log::debug!(target: "Timer", "System Timer Resolution: Min={:.1}ms, Max={:.1}ms, \
        Current={:.1}ms",
        f64::from(min) / 10000.0, f64::from(max) / 10000.0, f64::from(cur) / 10000.0);

    let enable_val = u8::from(enable);

    if enable {
        // SAFETY: `timeBeginPeriod` is called with a valid parameter.
        unsafe { windows_sys::Win32::Media::timeBeginPeriod(1) };
    } else {
        // Release the standard timeBeginPeriod if disabling
        // SAFETY: `timeEndPeriod` is called with the same period passed to `timeBeginPeriod`.
        unsafe { windows_sys::Win32::Media::timeEndPeriod(1) };
    }

    let mut new_cur = 0;
    // SAFETY: `new_cur` is a live local `u32`; `max` comes from `NtQueryTimerResolution`.
    let status = unsafe { nt_set(max, enable_val, &raw mut new_cur) };

    // `timeEndPeriod` above already cancelled our request, so cancelling it again finds nothing.
    let already_released = !enable && status.cast_unsigned() == STATUS_TIMER_RESOLUTION_NOT_SET;
    if status == 0 || already_released {
        log::info!(target: "Timer", "Timer resolution adjusted to {:.1}ms (enabled: {})",
            f64::from(new_cur) / 10000.0, enable);
        Ok(())
    } else {
        Err(PlatformError::Os(format!(
            "Failed to adjust the timer resolution (NTSTATUS: 0x{status:08X})"
        )))
    }
}
