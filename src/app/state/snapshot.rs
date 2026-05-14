use crate::core::config::models::MappingConfig;
use crate::drivers::TabletData;
use crate::engine::state::{LockRecoveryExt, SharedState};

/// An immutable, lock-free snapshot of the application state for a single UI frame.
#[derive(Clone, Debug)]
pub struct UiSnapshot {
    pub tablet_name: String,
    pub tablet_vid: u16,
    pub tablet_pid: u16,
    pub tablet_data: TabletData,
    pub config: MappingConfig,
    pub physical_size: (f32, f32),
    pub hardware_size: (f32, f32),
    pub stats: crate::drivers::DriverStats,
    pub packet_count: u32,
    pub is_first_run: bool,
}

impl UiSnapshot {
    /// Captures a complete state snapshot from the shared engine state.
    pub fn capture(shared: &SharedState) -> Self {
        use std::sync::atomic::Ordering;

        let device = shared
            .device_state
            .read()
            .unwrap_or_log("device_state")
            .clone();

        Self {
            tablet_name: device.name,
            tablet_vid: device.vid,
            tablet_pid: device.pid,
            tablet_data: shared
                .tablet_data
                .read()
                .unwrap_or_log("tablet_data")
                .clone(),
            config: shared.config.read().unwrap_or_log("config").clone(),
            physical_size: device.physical_size,
            hardware_size: device.hardware_size,
            stats: *shared.stats.read().unwrap_or_log("stats"),
            packet_count: shared.packet_count.load(Ordering::Relaxed),
            is_first_run: *shared.is_first_run.read().unwrap_or_log("is_first_run"),
        }
    }
}
