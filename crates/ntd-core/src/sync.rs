//! Lock helpers: what to do with a poisoned lock.
//!
//! A panic in a thread that holds a lock poisons it. These traits say, at each call site, whether
//! to keep the possibly corrupted data or to reset it to its default.

use std::sync::LockResult;

/// Extension trait for safe lock recovery.
pub trait LockRecoveryExt<T> {
    /// Extracts the guard but logs heavily that the state might be corrupted.
    /// Used for critical state where a reset would be destructive.
    fn unwrap_or_log(self, lock_name: &str) -> T;
}

impl<T> LockRecoveryExt<T> for LockResult<T> {
    fn unwrap_or_log(self, lock_name: &str) -> T {
        match self {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!(target: "State", "CRITICAL: Lock '{lock_name}' poisoned! Using \
                    potentially corrupted guard.");
                poisoned.into_inner()
            }
        }
    }
}

/// Extension trait for self-healing transient write locks.
pub trait WriteRecoverExt {
    /// The lock guard that [`WriteRecoverExt::unwrap_or_reset`] hands back.
    type Guard;
    /// Extracts the guard and explicitly overwrites the corrupted data with `Default::default()`.
    /// Used for transient state like tablet coordinates, stats, or device metadata.
    fn unwrap_or_reset(self, lock_name: &str) -> Self::Guard;
}

impl<'a, T: Default> WriteRecoverExt for LockResult<std::sync::RwLockWriteGuard<'a, T>> {
    type Guard = std::sync::RwLockWriteGuard<'a, T>;
    fn unwrap_or_reset(self, lock_name: &str) -> Self::Guard {
        match self {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::warn!(target: "State", "Write lock '{lock_name}' poisoned! Repairing by \
                    resetting to default state.");
                let mut guard = poisoned.into_inner();
                *guard = T::default();
                guard
            }
        }
    }
}

impl<'a, T: Default> WriteRecoverExt for LockResult<std::sync::MutexGuard<'a, T>> {
    type Guard = std::sync::MutexGuard<'a, T>;
    fn unwrap_or_reset(self, lock_name: &str) -> Self::Guard {
        match self {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::warn!(target: "State", "Mutex '{lock_name}' poisoned! Repairing by resetting \
                    to default state.");
                let mut guard = poisoned.into_inner();
                *guard = T::default();
                guard
            }
        }
    }
}
