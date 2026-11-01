//! What the machine is: its memory (`GlobalMemoryStatusEx`) and its name.
#![allow(
    unsafe_code,
    reason = "the memory query of Windows through windows-sys"
)]

use crate::SystemInfo;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

/// The installed memory and the computer name; Windows has no distribution name.
pub(crate) fn system_info() -> SystemInfo {
    SystemInfo {
        total_memory_bytes: total_memory_bytes(),
        distribution: None,
        hostname: std::env::var("COMPUTERNAME").ok(),
    }
}

fn total_memory_bytes() -> Option<u64> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };
    // SAFETY: `status` is valid for writes and `dwLength` is set, as the call requires.
    let success = unsafe { GlobalMemoryStatusEx(&raw mut status) };
    (success != 0).then_some(status.ullTotalPhys)
}
