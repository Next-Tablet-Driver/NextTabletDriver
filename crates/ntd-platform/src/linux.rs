//! The Linux side: an XDG autostart entry, `flock(2)` locks, and `nice`.

mod autostart;
mod lock;
mod priority;
mod system;

pub(crate) use autostart::{is_run_at_startup_registered, set_run_at_startup};
pub(crate) use lock::{LockHandle, try_acquire_lock};
pub(crate) use priority::raise_current_thread_priority;
pub(crate) use system::system_info;

use crate::PlatformError;

/// Only Windows needs a finer timer resolution.
#[allow(
    clippy::missing_const_for_fn,
    clippy::unnecessary_wraps,
    reason = "the same signature as Windows, where it does real work"
)]
pub(crate) fn set_high_resolution_timer(_enable: bool) -> Result<(), PlatformError> {
    Ok(())
}
