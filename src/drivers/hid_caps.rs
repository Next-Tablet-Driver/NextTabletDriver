//! Static HID input report length via `HidP_GetCaps`.
//!
//! Mirrors the static `device.InputReportLength` match filter that
//! `OpenTabletDriver` performs in `Driver.cs`, without depending on a live
//! data sample. Some tablets stay silent until the pen comes into
//! proximity, which makes a sample-read-based check unreliable at boot
//! time; this queries the OS-reported capability instead.

use std::ffi::CStr;
use windows_sys::Win32::Devices::HumanInterfaceDevice::{
    HIDP_CAPS, HidD_FreePreparsedData, HidD_GetPreparsedData, HidP_GetCaps,
};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, NTSTATUS};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileA, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

const HIDP_STATUS_SUCCESS: NTSTATUS = 0x0011_0000_u32.cast_signed();

/// Queries the OS-reported `InputReportByteLength` for a HID device path.
/// Returns `None` if the path can't be opened or the capability query fails.
#[must_use]
pub fn query_input_report_byte_length(path: &CStr) -> Option<usize> {
    // SAFETY: `path` is a NUL-terminated device path from hidapi's device
    // enumeration. Opened with zero access rights, sufficient for a
    // capability query and non-conflicting with any handle another
    // candidate probe or process may already hold on the device.
    let handle: HANDLE = unsafe {
        CreateFileA(
            path.as_ptr().cast(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let result = query_caps(handle);

    // SAFETY: `handle` was just returned by the successful `CreateFileA`
    // call above and is not used again after this point.
    unsafe {
        CloseHandle(handle);
    }

    result
}

fn query_caps(handle: HANDLE) -> Option<usize> {
    let mut preparsed_data: isize = 0;

    // SAFETY: `handle` is a valid, open HID device handle from `CreateFileA`.
    let ok = unsafe { HidD_GetPreparsedData(handle, &raw mut preparsed_data) };
    if !ok || preparsed_data == 0 {
        return None;
    }

    // SAFETY: `HIDP_CAPS` is a plain-old-data struct of integers; the
    // all-zero bit pattern is valid, and it is fully overwritten by
    // `HidP_GetCaps` below on success.
    let mut caps: HIDP_CAPS = unsafe { std::mem::zeroed() };
    // SAFETY: `preparsed_data` was just populated by the successful call above.
    let status = unsafe { HidP_GetCaps(preparsed_data, &raw mut caps) };

    // SAFETY: frees the preparsed data allocated by `HidD_GetPreparsedData` above.
    unsafe {
        HidD_FreePreparsedData(preparsed_data);
    }

    (status == HIDP_STATUS_SUCCESS).then_some(caps.InputReportByteLength as usize)
}
