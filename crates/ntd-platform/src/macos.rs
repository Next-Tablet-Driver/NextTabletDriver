//! macOS: not implemented in 3.0. Every call says [`PlatformError::Unsupported`], and the crate
//! still compiles for the target.

#![allow(
    clippy::missing_const_for_fn,
    clippy::unnecessary_wraps,
    reason = "the same signatures as the other systems, where these calls do real work"
)]

use crate::{NamedLock, PlatformError, UserPaths};

/// A lock that is never taken.
pub(crate) struct LockHandle;

pub(crate) fn set_run_at_startup(_enabled: bool, _paths: &UserPaths) -> Result<(), PlatformError> {
    Err(PlatformError::Unsupported)
}

pub(crate) fn is_run_at_startup_registered(_paths: &UserPaths) -> bool {
    false
}

pub(crate) fn try_acquire_lock(_lock: NamedLock) -> Option<LockHandle> {
    None
}

pub(crate) fn raise_current_thread_priority() -> Result<(), PlatformError> {
    Err(PlatformError::Unsupported)
}

pub(crate) fn set_high_resolution_timer(_enable: bool) -> Result<(), PlatformError> {
    Ok(())
}
