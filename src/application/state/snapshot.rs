use crate::engine::state::SharedState;
use ntd_core::config::models::MappingConfig;
use ntd_core::sync::LockRecoveryExt;
use ntd_tablets::TabletData;

/// An immutable, lock-free snapshot of the application state for a single UI frame.
#[derive(Clone, Debug)]
pub struct UiSnapshot {
    /// Name of the connected tablet.
    pub tablet_name: String,
    /// USB vendor ID of the connected tablet.
    pub tablet_vid: u16,
    /// USB product ID of the connected tablet.
    pub tablet_pid: u16,
    /// Latest packet parsed from the tablet.
    pub tablet_data: TabletData,
    /// The mapping configuration in effect.
    pub config: MappingConfig,
    /// Width and height of the tablet surface, in millimetres.
    pub physical_size: (f32, f32),
    /// Largest X and Y the sensor reports, in raw units.
    pub hardware_size: (f32, f32),
    /// Largest raw pressure the tablet reports.
    pub max_pressure: f32,
    /// Packet and timing statistics of the driver.
    pub stats: ntd_tablets::DriverStats,
    /// Number of packets processed since the engine started.
    pub packet_count: u32,
    /// Whether this is the first run of the application.
    pub is_first_run: bool,
    /// State of the input engine.
    pub engine_status: crate::engine::state::EngineStatus,
}

impl UiSnapshot {
    /// Captures a complete state snapshot from the shared engine state.
    pub fn capture(shared: &SharedState) -> Self {
        use std::sync::atomic::Ordering;

        let device = shared.device.read().unwrap_or_log("device").clone();

        Self {
            tablet_name: device.name,
            tablet_vid: device.vid,
            tablet_pid: device.pid,
            tablet_data: shared
                .pipeline
                .tablet_data
                .read()
                .unwrap_or_log("tablet_data")
                .clone(),
            config: shared.config.mapping.read().unwrap_or_log("config").clone(),
            physical_size: device.physical_size,
            hardware_size: device.hardware_size,
            max_pressure: device.max_pressure,
            stats: *shared.pipeline.stats.read().unwrap_or_log("stats"),
            packet_count: shared.pipeline.packet_count.load(Ordering::Relaxed),
            is_first_run: *shared
                .lifecycle
                .is_first_run
                .read()
                .unwrap_or_log("is_first_run"),
            engine_status: shared
                .lifecycle
                .engine_status
                .read()
                .unwrap_or_log("engine_status")
                .clone(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn capture_copies_the_device_and_counters() {
        let shared = SharedState::new();
        {
            let mut device = shared.device.write().unwrap();
            device.name = "Wacom CTL-472".to_string();
            device.vid = 0x056A;
            device.pid = 0x037A;
            device.physical_size = (152.0, 95.0);
            device.hardware_size = (15200.0, 9500.0);
            device.max_pressure = 2047.0;
        }
        shared.pipeline.packet_count.store(42, Ordering::Relaxed);
        *shared.lifecycle.is_first_run.write().unwrap() = true;

        let snapshot = UiSnapshot::capture(&shared);
        assert_eq!(snapshot.tablet_name, "Wacom CTL-472");
        assert_eq!((snapshot.tablet_vid, snapshot.tablet_pid), (0x056A, 0x037A));
        assert_eq!(snapshot.physical_size, (152.0, 95.0));
        assert_eq!(snapshot.hardware_size, (15200.0, 9500.0));
        assert_eq!(snapshot.max_pressure, 2047.0);
        assert_eq!(snapshot.packet_count, 42);
        assert!(snapshot.is_first_run);
    }

    #[test]
    fn capture_follows_the_live_configuration() {
        let shared = SharedState::new();
        shared.config.mapping.write().unwrap().active_area.w = 123.0;
        assert_eq!(UiSnapshot::capture(&shared).config.active_area.w, 123.0);
    }

    #[test]
    fn a_snapshot_is_detached_from_later_changes() {
        let shared = SharedState::new();
        let before = UiSnapshot::capture(&shared);
        shared.pipeline.packet_count.store(7, Ordering::Relaxed);
        shared.device.write().unwrap().name = "Changed".to_string();
        assert_eq!(before.packet_count, 0);
        assert_eq!(before.tablet_name, "No Tablet Detected");
    }
}
