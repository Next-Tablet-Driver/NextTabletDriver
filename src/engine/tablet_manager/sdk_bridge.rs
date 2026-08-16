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

    let mut device = shared.device_state.write().unwrap_or_reset("device_state");
    device.name = name.to_string();
    device.vid = snapshot.vid;
    device.pid = snapshot.pid;
    drop(device);

    let mut data = shared.tablet_data.write().unwrap_or_reset("tablet_data");
    data.is_connected = snapshot.is_connected;
    data.status = status_from_discriminant(snapshot.status);
    data.buttons = snapshot.buttons;
    data.eraser = snapshot.eraser;
    drop(data);

    *shared
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
        let mut config = shared.config.write().unwrap_or_log("config");
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
            .config_version
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
        let mut config = self.shared.config.write().unwrap_or_log("config");
        config.mode = mode;
        drop(config);
        self.shared.config_version.fetch_add(1, Ordering::SeqCst);
    }

    fn set_active_area(&self, area: ActiveArea) {
        let (phys_w, phys_h) = self
            .shared
            .device_state
            .read()
            .unwrap_or_log("device_state")
            .physical_size;

        let mut config = self.shared.config.write().unwrap_or_log("config");
        config.active_area = area;
        config.active_area.clamp_to_surface(phys_w, phys_h);
        drop(config);
        self.shared.config_version.fetch_add(1, Ordering::SeqCst);
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
    let device = shared.device_state.read().unwrap_or_log("device_state");
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
        config_version: shared.config_version.load(Ordering::Relaxed),
    };
    writer.publish(&state);
}
