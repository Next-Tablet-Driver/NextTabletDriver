//! Bridges this process's `SharedState` to/from `engine::interop`'s
//! SHM/command layer, so other processes (another SDK-embedded game, or a
//! second desktop instance) can mirror or drive this tablet's state.

use crate::core::config::models::{ActiveArea, DriverMode, MappingConfig};
use crate::drivers::{TabletData, TabletStatus};
use crate::engine::interop::command::CommandHandler;
use crate::engine::interop::shm::{DEVICE_NAME_CAPACITY, SdkPublicState, ShmWriter};
use crate::engine::pipeline::ProcessedFrame;
use crate::engine::state::{LockRecoveryExt, SharedState, WriteRecoverExt};
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Applies whatever config/state a remote HID owner published to this
/// process's own `SharedState`, so the desktop UI reflects live tablet data
/// even when another process (another SDK-embedded game, or a second
/// desktop instance) is the one actually driving the device.
///
/// Local config writes made through the desktop UI while in reader mode are
/// intentionally out of scope here. The desktop UI has no notion yet of
/// "control the remote owner's device" versus "edit my own settings"; that
/// distinction belongs to a UI-level change, not this wiring.
pub(super) fn apply_shm_snapshot(
    shared: &Arc<SharedState>,
    snapshot: &SdkPublicState,
    last_config_version: &mut Option<u32>,
) {
    let name_len = (snapshot.device_name_len as usize).min(snapshot.device_name.len());
    let name = snapshot
        .device_name
        .get(..name_len)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .filter(|s| !s.is_empty())
        .unwrap_or("No Tablet Detected");

    let mut device = shared.device.write().unwrap_or_reset("device");
    device.name = name.to_string();
    device.vid = snapshot.vid;
    device.pid = snapshot.pid;
    drop(device);

    let mut data = shared
        .pipeline
        .tablet_data
        .write()
        .unwrap_or_reset("tablet_data");
    data.is_connected = snapshot.is_connected;
    data.status = status_from_discriminant(snapshot.status);
    data.buttons = snapshot.buttons;
    data.eraser = snapshot.eraser;
    drop(data);

    *shared
        .pipeline
        .processed_frame
        .write()
        .unwrap_or_reset("processed_frame") = ProcessedFrame {
        u: snapshot.u,
        v: snapshot.v,
        screen_x: snapshot.screen_x,
        screen_y: snapshot.screen_y,
        is_down: snapshot.is_down,
        pressure: snapshot.pressure,
        tilt_x: snapshot.tilt_x,
        tilt_y: snapshot.tilt_y,
    };

    if *last_config_version != Some(snapshot.config_version) {
        *last_config_version = Some(snapshot.config_version);
        let mut config = shared.config.mapping.write().unwrap_or_log("config");
        config.mode = if snapshot.mode == 1 {
            DriverMode::Relative
        } else {
            DriverMode::Absolute
        };
        config.active_area = ActiveArea {
            x: snapshot.active_area_x,
            y: snapshot.active_area_y,
            w: snapshot.active_area_w,
            h: snapshot.active_area_h,
            rotation: snapshot.active_area_rotation,
        };
        drop(config);
        shared
            .config
            .version
            .store(snapshot.config_version, Ordering::SeqCst);
    }
}

/// Maps a raw [`SdkPublicState::status`] byte back to [`TabletStatus`].
///
/// Mirrors `TabletStatus`'s declaration order in `drivers::models`. The
/// discriminant was produced on the publishing side via a plain `as u8` cast,
/// so this must stay in sync with that enum's variant order.
const fn status_from_discriminant(byte: u8) -> TabletStatus {
    match byte {
        1 => TabletStatus::OutOfRange,
        2 => TabletStatus::Hover,
        3 => TabletStatus::Contact,
        4 => TabletStatus::Active,
        5 => TabletStatus::Eraser,
        6 => TabletStatus::Pen,
        7 => TabletStatus::Touch,
        8 => TabletStatus::Aux,
        9 => TabletStatus::Rotation,
        10 => TabletStatus::Tool,
        11 => TabletStatus::Mouse,
        _ => TabletStatus::Disconnected,
    }
}

/// Applies a reader's [`Request`](crate::engine::interop::command::Request)
/// to this owner's real `SharedState`, through the exact same
/// validation/write path a local caller (the desktop UI) would use.
pub(super) struct DesktopCommandHandler {
    pub(super) shared: Arc<SharedState>,
}

impl CommandHandler for DesktopCommandHandler {
    fn set_mode(&self, mode: DriverMode) {
        let mut config = self.shared.config.mapping.write().unwrap_or_log("config");
        config.mode = mode;
        drop(config);
        self.shared.config.version.fetch_add(1, Ordering::SeqCst);
    }

    fn set_active_area(&self, area: ActiveArea) {
        let (phys_w, phys_h) = self
            .shared
            .device
            .read()
            .unwrap_or_log("device")
            .physical_size;

        let mut config = self.shared.config.mapping.write().unwrap_or_log("config");
        config.active_area = area;
        config.active_area.clamp_to_surface(phys_w, phys_h);
        drop(config);
        self.shared.config.version.fetch_add(1, Ordering::SeqCst);
    }
}

/// Builds an [`SdkPublicState`] snapshot from this iteration's live values
/// and publishes it, so every reader process sees this owner's tablet data.
pub(super) fn publish_shm_state(
    writer: &ShmWriter,
    shared: &Arc<SharedState>,
    data: &TabletData,
    config: &MappingConfig,
    frame: &ProcessedFrame,
) {
    let device = shared.device.read().unwrap_or_log("device");
    let mut device_name = [0u8; DEVICE_NAME_CAPACITY];
    let name_bytes = device.name.as_bytes();
    let name_len = name_bytes.len().min(device_name.len());
    if let (Some(dest), Some(src)) = (device_name.get_mut(..name_len), name_bytes.get(..name_len)) {
        dest.copy_from_slice(src);
    }
    let vid = device.vid;
    let pid = device.pid;
    drop(device);

    let state = SdkPublicState {
        is_connected: data.is_connected,
        status: data.status as u8,
        u: frame.u,
        v: frame.v,
        screen_x: frame.screen_x,
        screen_y: frame.screen_y,
        pressure: frame.pressure,
        tilt_x: frame.tilt_x,
        tilt_y: frame.tilt_y,
        buttons: data.buttons,
        is_down: frame.is_down,
        eraser: data.eraser,
        device_name,
        device_name_len: name_len as u32,
        vid,
        pid,
        mode: match config.mode {
            DriverMode::Absolute => 0,
            DriverMode::Relative => 1,
        },
        active_area_x: config.active_area.x,
        active_area_y: config.active_area.y,
        active_area_w: config.active_area.w,
        active_area_h: config.active_area.h,
        active_area_rotation: config.active_area.rotation,
        config_version: shared.config.version.load(Ordering::Relaxed),
    };
    writer.publish(&state);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn snapshot() -> SdkPublicState {
        let mut device_name = [0u8; DEVICE_NAME_CAPACITY];
        device_name[..5].copy_from_slice(b"Wacom");
        SdkPublicState {
            is_connected: true,
            status: 3,
            u: 0.25,
            v: 0.75,
            screen_x: 480.0,
            screen_y: 810.0,
            pressure: 1234,
            tilt_x: -5,
            tilt_y: 7,
            buttons: 2,
            is_down: true,
            eraser: true,
            device_name,
            device_name_len: 5,
            vid: 0x056A,
            pid: 0x037A,
            mode: 1,
            active_area_x: 50.0,
            active_area_y: 30.0,
            active_area_w: 80.0,
            active_area_h: 40.0,
            active_area_rotation: 10.0,
            config_version: 5,
        }
    }

    #[test]
    fn every_status_survives_the_round_trip_through_its_discriminant() {
        use TabletStatus::*;
        for status in [
            Disconnected,
            OutOfRange,
            Hover,
            Contact,
            Active,
            Eraser,
            Pen,
            Touch,
            Aux,
            Rotation,
            Tool,
            Mouse,
        ] {
            assert_eq!(status_from_discriminant(status as u8), status);
        }
    }

    #[test]
    fn unknown_discriminants_read_as_disconnected() {
        assert_eq!(status_from_discriminant(12), TabletStatus::Disconnected);
        assert_eq!(status_from_discriminant(255), TabletStatus::Disconnected);
    }

    #[test]
    fn a_snapshot_is_mirrored_into_the_shared_state() {
        let shared = Arc::new(SharedState::new());
        let mut version = None;
        apply_shm_snapshot(&shared, &snapshot(), &mut version);

        let device = shared.device.read().unwrap().clone();
        assert_eq!(
            (device.name.as_str(), device.vid, device.pid),
            ("Wacom", 0x056A, 0x037A)
        );

        let data = shared.pipeline.tablet_data.read().unwrap().clone();
        assert!(data.is_connected);
        assert_eq!(data.status, TabletStatus::Contact);
        assert_eq!(data.buttons, 2);
        assert!(data.eraser);

        let frame = *shared.pipeline.processed_frame.read().unwrap();
        assert_eq!((frame.u, frame.v), (0.25, 0.75));
        assert_eq!((frame.screen_x, frame.screen_y), (480.0, 810.0));
        assert_eq!((frame.pressure, frame.tilt_x, frame.tilt_y), (1234, -5, 7));
        assert!(frame.is_down);

        let config = shared.config.mapping.read().unwrap().clone();
        assert_eq!(config.mode, DriverMode::Relative);
        assert_eq!(
            (
                config.active_area.x,
                config.active_area.y,
                config.active_area.w,
                config.active_area.h
            ),
            (50.0, 30.0, 80.0, 40.0)
        );
        assert_eq!(config.active_area.rotation, 10.0);
        assert_eq!(shared.config.version.load(Ordering::SeqCst), 5);
        assert_eq!(version, Some(5));
    }

    #[test]
    fn the_config_is_only_applied_when_its_version_changes() {
        let shared = Arc::new(SharedState::new());
        let mut version = None;
        apply_shm_snapshot(&shared, &snapshot(), &mut version);

        // Same version, different area: the live data follows, the config does not.
        let mut same = snapshot();
        same.active_area_w = 10.0;
        same.pressure = 99;
        apply_shm_snapshot(&shared, &same, &mut version);
        assert_eq!(shared.config.mapping.read().unwrap().active_area.w, 80.0);
        assert_eq!(shared.pipeline.processed_frame.read().unwrap().pressure, 99);

        let mut newer = same;
        newer.config_version = 6;
        newer.mode = 0;
        apply_shm_snapshot(&shared, &newer, &mut version);
        let config = shared.config.mapping.read().unwrap().clone();
        assert_eq!(config.active_area.w, 10.0);
        assert_eq!(config.mode, DriverMode::Absolute);
        assert_eq!(shared.config.version.load(Ordering::SeqCst), 6);
        assert_eq!(version, Some(6));
    }

    #[test]
    fn a_missing_or_corrupt_device_name_falls_back_to_the_placeholder() {
        let shared = Arc::new(SharedState::new());

        let mut empty = snapshot();
        empty.device_name_len = 0;
        apply_shm_snapshot(&shared, &empty, &mut None);
        assert_eq!(shared.device.read().unwrap().name, "No Tablet Detected");

        let mut invalid = snapshot();
        invalid.device_name[..2].copy_from_slice(&[0xFF, 0xFE]);
        invalid.device_name_len = 2;
        apply_shm_snapshot(&shared, &invalid, &mut None);
        assert_eq!(shared.device.read().unwrap().name, "No Tablet Detected");
    }

    #[test]
    fn an_oversized_name_length_is_clamped_to_the_buffer() {
        let shared = Arc::new(SharedState::new());
        let mut oversized = snapshot();
        oversized.device_name = [b'a'; DEVICE_NAME_CAPACITY];
        oversized.device_name_len = u32::MAX;
        apply_shm_snapshot(&shared, &oversized, &mut None);
        assert_eq!(
            shared.device.read().unwrap().name.len(),
            DEVICE_NAME_CAPACITY
        );
    }

    #[test]
    fn a_remote_mode_change_updates_the_config_and_bumps_the_version() {
        let shared = Arc::new(SharedState::new());
        let handler = DesktopCommandHandler {
            shared: Arc::clone(&shared),
        };
        let before = shared.config.version.load(Ordering::SeqCst);
        handler.set_mode(DriverMode::Relative);
        assert_eq!(
            shared.config.mapping.read().unwrap().mode,
            DriverMode::Relative
        );
        assert_eq!(shared.config.version.load(Ordering::SeqCst), before + 1);
    }

    #[test]
    fn a_remote_active_area_is_clamped_to_the_tablet_surface() {
        let shared = Arc::new(SharedState::new());
        shared.device.write().unwrap().physical_size = (100.0, 60.0);
        let handler = DesktopCommandHandler {
            shared: Arc::clone(&shared),
        };
        let before = shared.config.version.load(Ordering::SeqCst);

        handler.set_active_area(ActiveArea {
            x: 500.0,
            y: 500.0,
            w: 300.0,
            h: 300.0,
            rotation: 0.0,
        });

        let area = shared.config.mapping.read().unwrap().active_area;
        assert_eq!((area.w, area.h), (100.0, 60.0));
        assert_eq!((area.x, area.y), (50.0, 30.0));
        assert_eq!(shared.config.version.load(Ordering::SeqCst), before + 1);
    }

    #[test]
    fn publishing_makes_the_live_state_visible_to_readers() {
        use crate::engine::interop::shm::ShmReader;
        let writer = ShmWriter::create().expect("writer should create the segment");
        let reader = ShmReader::open().expect("reader should open the same segment");

        let shared = Arc::new(SharedState::new());
        {
            let mut device = shared.device.write().unwrap();
            device.name = "Wacom".to_string();
            device.vid = 0x056A;
            device.pid = 0x037A;
        }
        shared.config.version.store(9, Ordering::Relaxed);
        let data = TabletData {
            is_connected: true,
            status: TabletStatus::Contact,
            buttons: 2,
            eraser: true,
            ..TabletData::default()
        };
        let config = MappingConfig {
            mode: DriverMode::Relative,
            ..MappingConfig::default()
        };
        let frame = ProcessedFrame {
            u: 0.25,
            v: 0.75,
            screen_x: 480.0,
            screen_y: 810.0,
            is_down: true,
            pressure: 1234,
            tilt_x: -5,
            tilt_y: 7,
        };

        publish_shm_state(&writer, &shared, &data, &config, &frame);

        let published = reader.read().unwrap();
        assert!(published.is_connected);
        assert_eq!(published.status, TabletStatus::Contact as u8);
        assert_eq!((published.buttons, published.eraser), (2, true));
        assert_eq!(&published.device_name[..5], b"Wacom");
        assert_eq!(published.device_name_len, 5);
        assert_eq!((published.vid, published.pid), (0x056A, 0x037A));
        assert_eq!(published.mode, 1);
        assert_eq!((published.u, published.v), (0.25, 0.75));
        assert_eq!((published.screen_x, published.screen_y), (480.0, 810.0));
        assert_eq!(
            (published.pressure, published.tilt_x, published.tilt_y),
            (1234, -5, 7)
        );
        assert!(published.is_down);
        assert_eq!(published.config_version, 9);
    }

    #[test]
    fn a_device_name_longer_than_the_segment_is_truncated() {
        use crate::engine::interop::shm::ShmReader;
        let writer = ShmWriter::create().expect("writer should create the segment");
        let reader = ShmReader::open().expect("reader should open the same segment");
        let shared = Arc::new(SharedState::new());
        shared.device.write().unwrap().name = "x".repeat(DEVICE_NAME_CAPACITY * 2);

        publish_shm_state(
            &writer,
            &shared,
            &TabletData::default(),
            &MappingConfig::default(),
            &ProcessedFrame::default(),
        );

        let published = reader.read().unwrap();
        assert_eq!(published.device_name_len as usize, DEVICE_NAME_CAPACITY);
        assert!(published.device_name.iter().all(|b| *b == b'x'));
    }
}
