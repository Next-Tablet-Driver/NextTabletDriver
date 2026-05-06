//! # Thread-Safe Application State
//!
//! This module defines the `SharedState` structure, which provides a bridge
//! between the high-frequency background `engine` threads and the 60Hz GUI threads.

use crate::core::config::models::MappingConfig;
use crate::drivers::TabletData;
use std::sync::atomic::AtomicU32;
use std::sync::{LockResult, RwLock};

/// Extension trait to handle poisoned locks gracefully.
pub trait LockResultExt<T> {
    fn ignore_poison(self) -> T;
}

impl<T> LockResultExt<T> for LockResult<T> {
    fn ignore_poison(self) -> T {
        match self {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!(target: "State", "Lock poisoned! Another thread panicked while holding this lock. Recovering guard...");
                poisoned.into_inner()
            }
        }
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(MappingConfig::default()),
            config_version: AtomicU32::new(0),
            tablet_data: RwLock::new(TabletData::default()),
            tablet_name: RwLock::new("No Tablet Detected".to_string()),
            tablet_vid: RwLock::new(0),
            tablet_pid: RwLock::new(0),
            physical_size: RwLock::new((160.0, 100.0)),
            hardware_size: RwLock::new((32767.0, 32767.0)),
            is_first_run: RwLock::new(false),
            packet_count: AtomicU32::new(0),
            stats: RwLock::new(crate::drivers::DriverStats::default()),

        }
    }

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
    /// The marketing/USB name of the connected tablet device.
    pub tablet_name: RwLock<String>,
    /// USB Vendor ID of the connected device.
    pub tablet_vid: RwLock<u16>,
    /// USB Product ID of the connected device.
    pub tablet_pid: RwLock<u16>,
    /// The immutable physical dimensions of the active tablet in millimeters (Width, Height).
    pub physical_size: RwLock<(f32, f32)>,
    /// The maximum raw hardware values the tablet can output (Max_X, Max_Y).
    pub hardware_size: RwLock<(f32, f32)>,
    /// Flag indicating if the user has never launched the application before (triggers welcome modal).
    pub is_first_run: RwLock<bool>,
    /// A rapidly incrementing counter of USB packets received, used by the UI to calculate real-time Hz.
    pub packet_count: AtomicU32,
    /// Tracking statistics for developer debugging (e.g., dropped packets, parse errors).
    pub stats: RwLock<crate::drivers::DriverStats>,
}
