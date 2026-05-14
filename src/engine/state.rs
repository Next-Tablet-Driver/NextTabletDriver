//! # Thread-Safe Application State
//!
//! This module defines the `SharedState` structure, which provides a bridge
//! between the high-frequency background `engine` threads and the 60Hz GUI threads.

use crate::core::config::models::MappingConfig;
use crate::drivers::TabletData;
use std::sync::atomic::AtomicU32;
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

#[derive(Clone, Debug, PartialEq)]
pub enum EngineStatus {
    Running,
    Failed(String),
}

impl Default for EngineStatus {
    fn default() -> Self {
        Self::Running
    }
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
            shutdown_requested: std::sync::atomic::AtomicBool::new(false),
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
    pub shutdown_requested: std::sync::atomic::AtomicBool,
}
