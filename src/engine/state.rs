//! # Thread-Safe Application State
//!
//! This module defines the `SharedState` structure, which provides a bridge
//! between the high-frequency background `engine` threads and the 60Hz GUI threads.

use crate::core::config::models::MappingConfig;
use crate::drivers::TabletData;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{LockResult, RwLock};

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
                log::error!(target: "State", "CRITICAL: Lock '{lock_name}' poisoned! Using potentially corrupted guard.");
                poisoned.into_inner()
            }
        }
    }
}

/// Extension trait for self-healing transient write locks.
pub trait WriteRecoverExt {
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
                log::warn!(target: "State", "Write lock '{lock_name}' poisoned! Repairing by resetting to default state.");
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
                log::warn!(target: "State", "Mutex '{lock_name}' poisoned! Repairing by resetting to default state.");
                let mut guard = poisoned.into_inner();
                *guard = T::default();
                guard
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum EngineStatus {
    #[default]
    Running,
    Failed(String),
}

/// An atomic snapshot of device properties.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceState {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub physical_size: (f32, f32),
    pub hardware_size: (f32, f32),
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            name: "No Tablet Detected".to_string(),
            vid: 0,
            pid: 0,
            physical_size: (160.0, 100.0),
            hardware_size: (32767.0, 32767.0),
        }
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: RwLock::new(MappingConfig::default()),
            config_version: AtomicU32::new(0),
            tablet_data: RwLock::new(TabletData::default()),
            device_state: RwLock::new(DeviceState::default()),
            is_first_run: RwLock::new(false),
            packet_count: AtomicU32::new(0),
            stats: RwLock::new(crate::drivers::DriverStats::default()),
            engine_status: RwLock::new(EngineStatus::default()),
            shutdown_requested: AtomicBool::new(false),
            is_visible: AtomicBool::new(true),
            reload_requested: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub fn test_default() -> Self {
        Self::new()
    }
}

/// The central thread-safe state store for the application.
///
/// Due to the disparate update rates of the background processing engine (often 100-1000Hz)
/// and the user interface (locked to vsync/60Hz), all shared data is wrapped in `RwLock`
/// or atomic types to ensure memory safety without creating massive mutex contention.
pub struct SharedState {
    /// The currently active settings (mapping area, filters, network, etc).
    pub config: RwLock<MappingConfig>,
    /// An atomic counter incremented by the UI whenever `config` is modified.
    /// The background thread checks this to avoid acquiring read-locks continuously.
    pub config_version: AtomicU32,
    /// The most recent normalized packet from the tablet (X, Y, Pressure, Pen Buttons).
    pub tablet_data: RwLock<TabletData>,
    /// Cohesive properties of the active device (name, vid, pid, sizes).
    pub device_state: RwLock<DeviceState>,
    /// Flag indicating if the user has never launched the application before (triggers welcome modal).
    pub is_first_run: RwLock<bool>,
    /// A rapidly incrementing counter of USB packets received, used by the UI to calculate real-time Hz.
    pub packet_count: AtomicU32,
    /// Tracking statistics for developer debugging (e.g., dropped packets, parse errors).
    pub stats: RwLock<crate::drivers::DriverStats>,
    /// Status of the background polling engine (e.g. HID API initialization failure).
    pub engine_status: RwLock<EngineStatus>,
    /// Flag indicating that the application is shutting down and threads should terminate.
    pub shutdown_requested: AtomicBool,
    /// Flag indicating whether the GUI window is currently visible.
    /// When `false` (minimized to tray), background threads skip UI-only
    /// work (channel sends, snapshot captures) to minimize idle CPU usage.
    pub is_visible: AtomicBool,
    /// Flag indicating that the user requested a full HID engine reload from the tray menu.
    pub reload_requested: AtomicBool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, RwLock};
    use std::thread;

    #[test]
    fn mutex_unwrap_or_reset_on_poisoned_lock() {
        let m = Arc::new(Mutex::new(String::from("dirty")));
        let m2 = Arc::clone(&m);
        let handle = thread::spawn(move || {
            let _guard = m2.lock().unwrap();
            // Panic while holding the lock to poison it
            panic!("poison mutex");
        });
        let _ = handle.join(); // ignore the panic from the spawned thread

        let lock_result = m.lock();
        // This should recover by resetting the inner value to Default
        let guard = lock_result.unwrap_or_reset("test_mutex");
        assert_eq!(*guard, String::default());

        // Ensure the underlying data is actually reset
        drop(guard);
        // The lock remains poisoned in the OS-level primitive; use unwrap_or_log to access the inner value.
        let inner = m.lock().unwrap_or_log("test_mutex_check");
        assert_eq!(*inner, String::default());
    }

    #[test]
    fn rwlock_write_unwrap_or_reset_on_poison() {
        let r = Arc::new(RwLock::new(vec![1, 2, 3]));
        let r2 = Arc::clone(&r);
        let handle = thread::spawn(move || {
            let _g = r2.write().unwrap();
            panic!("poison rwlock");
        });
        let _ = handle.join();

        let res = r.write();
        let guard = res.unwrap_or_reset("test_rwlock");
        assert!(guard.is_empty());
    }

    #[test]
    fn mutex_unwrap_or_log_on_poison() {
        let m = Arc::new(Mutex::new(42u32));
        let m2 = Arc::clone(&m);
        let handle = thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("poison");
        });
        let _ = handle.join();

        let res = m.lock();
        let guard = res.unwrap_or_log("test_mutex_log");
        // The value should still be accessible (we didn't modify it before panic)
        assert_eq!(*guard, 42u32);
    }
}
