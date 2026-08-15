//! Windows shared memory backing for the seqlock segment, via a named
//! page-file-backed file mapping (`CreateFileMappingW`/`MapViewOfFile`).

use std::ffi::c_void;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
    PAGE_READWRITE, UnmapViewOfFile,
};

const SEGMENT_NAME: &str = "Local\\NextTabletDriver_State_v1";

fn wide_name() -> Vec<u16> {
    SEGMENT_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

/// Owns both the mapping object handle and the mapped view. Either
/// `create_mapping` or `open_mapping` produces one; dropping it unmaps the
/// view and closes the handle, but the segment's last published contents
/// persist for the next process to map it (page-file-backed, not
/// process-owned memory).
pub struct Mapping {
    handle: HANDLE,
    view: *mut c_void,
}

// SAFETY: the wrapped handle/pointer have no thread-affinity requirements;
// `ShmSegment`'s own internal synchronization (seqlock) is what guards the
// memory they point to, not any property of this type.
unsafe impl Send for Mapping {}
// SAFETY: see above — safe concurrent access to the pointee is guaranteed by
// the seqlock protocol in `super`, not by this type.
unsafe impl Sync for Mapping {}

impl Mapping {
    pub const fn as_ptr(&self) -> *mut c_void {
        self.view
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: `self.view` was returned by a successful `MapViewOfFile`
        // and has not been unmapped yet.
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: self.view });
        }
        // SAFETY: `self.handle` is a valid, still-open mapping handle.
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

fn map_view(handle: HANDLE, size: usize) -> Option<Mapping> {
    // SAFETY: `handle` is a valid file-mapping handle for the duration of
    // this call; requesting a view of exactly `size` bytes at offset 0 stays
    // within the mapping's committed size.
    let view = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size) };
    if view.Value.is_null() {
        // SAFETY: `handle` is valid and owned by this call path; the mapping
        // failed, so it must be closed here to avoid leaking it.
        unsafe {
            CloseHandle(handle);
        }
        return None;
    }
    Some(Mapping {
        handle,
        view: view.Value,
    })
}

/// Creates (or takes over) the well-known named mapping, sized to hold one
/// `ShmSegment`. Windows reference-counts the underlying pages: if another
/// process already created this mapping, this call opens the same one.
pub fn create_mapping(size: usize) -> Option<Mapping> {
    let name = wide_name();
    let size_u64 = size as u64;
    let size_high = (size_u64 >> 32) as u32;
    let size_low = (size_u64 & 0xFFFF_FFFF) as u32;

    // SAFETY: `INVALID_HANDLE_VALUE` requests a page-file-backed mapping (no
    // real file); `name` is a valid, NUL-terminated UTF-16 buffer kept alive
    // for the duration of the call; no security attributes are needed since
    // the mapping is local-session-scoped by its `Local\` name prefix.
    let handle = unsafe {
        CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            std::ptr::null(),
            PAGE_READWRITE,
            size_high,
            size_low,
            name.as_ptr(),
        )
    };
    if handle.is_null() {
        return None;
    }
    map_view(handle, size)
}

/// Opens the well-known named mapping if it already exists; never creates
/// it. Used by readers, which should never bring the segment into existence
/// themselves — only the current HID owner publishes into it.
pub fn open_mapping(size: usize) -> Option<Mapping> {
    let name = wide_name();
    // SAFETY: `name` is a valid, NUL-terminated UTF-16 buffer kept alive for
    // the duration of the call.
    let handle = unsafe {
        windows_sys::Win32::System::Memory::OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, name.as_ptr())
    };
    if handle.is_null() {
        return None;
    }
    map_view(handle, size)
}
