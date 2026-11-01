//! Any other system: not implemented. No segment can be mapped, so
//! [`crate::shm::ShmWriter::create`] and [`crate::shm::ShmReader::open`] give nothing, and the
//! crate still compiles for the target.

use std::ffi::c_void;

/// A mapping that never exists.
pub(crate) struct Mapping(());

impl Mapping {
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        std::ptr::null_mut()
    }
}

pub(crate) const fn create_mapping(_size: usize) -> Option<Mapping> {
    None
}

pub(crate) const fn open_mapping(_size: usize) -> Option<Mapping> {
    None
}
