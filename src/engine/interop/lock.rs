//! # HID Owner Lock
//!
//! Named, machine-wide, non-blocking lock used to elect exactly one process as
//! the "HID owner": the one allowed to actually open the tablet's HID device.
//! Holding [`HidOwnerGuard`] means this process is the owner; every other
//! process (another SDK-embedded game, or the desktop app if it's also
//! running) fails to acquire it and instead reads the owner's published state.
//!
//! The lock is released automatically by the OS if the owning process dies
//! (crash, kill, normal exit), so a non-owner retrying [`try_acquire_hid_owner`]
//! is promoted the moment the previous owner disappears. No stale-lock
//! cleanup logic is needed on either platform.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as os;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as os;

/// Held by whichever process currently owns the real HID device.
///
/// The wrapped platform handle is never read again after acquisition; it's
/// kept purely so that dropping this guard (process exit, or an explicit
/// `drop`) releases the lock via its `Drop` impl.
pub struct HidOwnerGuard(#[allow(dead_code)] os::OwnerHandle);

/// Attempts to become the HID owner.
///
/// Non-blocking: returns `None` immediately if another process already holds
/// the lock, rather than waiting for it to be released. Callers that fail to
/// acquire should fall back to reader mode (`engine::interop::shm`) and retry
/// periodically to detect promotion when the current owner exits.
#[must_use]
pub fn try_acquire_hid_owner() -> Option<HidOwnerGuard> {
    os::try_acquire().map(HidOwnerGuard)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Exercises the "only one owner at a time" guarantee across threads.
    ///
    /// Uses a second OS thread (not just a second call on this thread) because
    /// both the Windows named-mutex and Linux `flock` primitives are
    /// per-thread/per-open-file-description: a second acquire on the *same*
    /// thread would spuriously succeed (recursive mutex ownership on Windows)
    /// instead of exercising real contention.
    ///
    /// Note: this contends on the real machine-wide lock name, so it will
    /// spuriously fail if a desktop instance or another SDK consumer happens
    /// to be running as the owner at the same time this test runs.
    #[test]
    fn only_one_owner_at_a_time() {
        let guard = try_acquire_hid_owner();
        assert!(guard.is_some(), "first acquire should succeed");

        let second = std::thread::spawn(try_acquire_hid_owner)
            .join()
            .expect("spawned thread should not panic");
        assert!(
            second.is_none(),
            "second acquire from another thread should fail while the first guard is held"
        );

        drop(guard);

        let third = std::thread::spawn(try_acquire_hid_owner)
            .join()
            .expect("spawned thread should not panic");
        assert!(
            third.is_some(),
            "acquire should succeed again once the owner releases the guard"
        );
    }
}
