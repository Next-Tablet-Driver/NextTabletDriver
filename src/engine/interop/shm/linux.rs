//! Linux shared memory backing for the seqlock segment, via a POSIX shared
//! memory object (`shm_open`/`mmap`).

use nix::fcntl::OFlag;
use nix::sys::mman::{MapFlags, ProtFlags, mmap, munmap, shm_open};
use nix::sys::stat::Mode;
use nix::unistd::ftruncate;
use std::ffi::c_void;
use std::num::NonZeroUsize;
use std::os::fd::AsFd;
use std::ptr::NonNull;

const SEGMENT_NAME: &str = "/ntd_state_v1";

/// Read-write for the owning user, read-only for group/other — mirrors the
/// permissiveness of the Windows `Local\` mapping, which any local session
/// process can open.
fn segment_mode() -> Mode {
    Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IROTH
}

/// Owns the `mmap`ed view. The backing shared memory object itself outlives
/// this mapping (never `shm_unlink`ed here) so the next process to open the
/// same name sees the last published contents, matching the Windows side's
/// persistence behavior.
pub struct Mapping {
    view: NonNull<c_void>,
    size: usize,
}

// SAFETY: the wrapped pointer has no thread-affinity requirements; safe
// concurrent access to the pointee is guaranteed by the seqlock protocol in
// `super`, not by this type.
unsafe impl Send for Mapping {}
// SAFETY: see above.
unsafe impl Sync for Mapping {}

impl Mapping {
    pub const fn as_ptr(&self) -> *mut c_void {
        self.view.as_ptr()
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: `self.view` was returned by a successful `mmap` of
        // `self.size` bytes and has not been unmapped yet.
        unsafe {
            let _ = munmap(self.view, self.size);
        }
    }
}

fn map_fd<F: AsFd>(fd: F, size: usize) -> Option<Mapping> {
    let len = NonZeroUsize::new(size)?;
    // SAFETY: `fd` refers to a shared memory object already sized to at
    // least `size` bytes via `ftruncate`; requesting a shared, read-write
    // mapping of the whole object at a kernel-chosen address is safe.
    let view = unsafe {
        mmap(
            None,
            len,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
            fd,
            0,
        )
    }
    .ok()?;
    Some(Mapping { view, size })
}

/// Creates (or takes over) the well-known shared memory object, sized to
/// hold one `ShmSegment`.
pub fn create_mapping(size: usize) -> Option<Mapping> {
    let fd = shm_open(SEGMENT_NAME, OFlag::O_CREAT | OFlag::O_RDWR, segment_mode()).ok()?;
    ftruncate(&fd, size.try_into().ok()?).ok()?;
    map_fd(fd, size)
}

/// Opens the well-known shared memory object if it already exists; never
/// creates it. Used by readers, which should never bring the segment into
/// existence themselves — only the current HID owner publishes into it.
pub fn open_mapping(size: usize) -> Option<Mapping> {
    let fd = shm_open(SEGMENT_NAME, OFlag::O_RDWR, segment_mode()).ok()?;
    map_fd(fd, size)
}
