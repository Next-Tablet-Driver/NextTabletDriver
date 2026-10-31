//! The Windows side: registry autostart, a named mutex, `SetThreadPriority` and the timer
//! resolution of `ntdll`.

mod autostart;
mod lock;
mod priority;
mod timer;

pub(crate) use autostart::{is_run_at_startup_registered, set_run_at_startup};
pub(crate) use lock::{LockHandle, try_acquire_lock};
pub(crate) use priority::raise_current_thread_priority;
pub(crate) use timer::set_high_resolution_timer;
