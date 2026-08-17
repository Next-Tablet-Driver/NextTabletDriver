//! # Embedded Engine Loop
//!
//! The background thread the SDK spawns inside the host process (a game, a
//! Blender plugin, ...) to drive tablet input on its own, without a desktop
//! `NextTabletDriver` instance. Adapted from the desktop app's
//! `engine::tablet_manager` detect/parse/[`Pipeline::process`]/hot-reload
//! algorithm, minus everything that's desktop-only: no `engine::injector`
//! (the host reads raw pressure/tilt straight from [`SharedState`] instead of
//! losing it through an OS input event), no telemetry, no UI channel.
//!
//! Coexistence with the desktop app (or other SDK-embedded games) is handled
//! by `engine::interop`, exactly as documented there: this loop becomes
//! either the HID owner or a reader of whichever process already is one.

use next_tablet_driver::core::config::models::{ActiveArea, DriverMode, MappingConfig};
use next_tablet_driver::drivers::{TabletData, TabletStatus, detect_tablet};
use next_tablet_driver::engine::interop::command::{CommandHandler, CommandListener};
use next_tablet_driver::engine::interop::lock::try_acquire_hid_owner;
use next_tablet_driver::engine::interop::shm::{
    DEVICE_NAME_CAPACITY, SdkPublicState, ShmReader, ShmWriter,
};
use next_tablet_driver::engine::pipeline::{Pipeline, ProcessedFrame};
use next_tablet_driver::engine::state::{LockRecoveryExt, SharedState, WriteRecoverExt};
use next_tablet_driver::filters::FilterPipeline;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Spawns the background thread that drives `shared` for as long as
/// `shared.shutdown_requested` stays false.
///
/// Mirrors the desktop app's `run_manager` retry loop: a panic inside one
/// iteration is caught and logged rather than killing the thread outright,
/// and the loop restarts after a short delay. `is_owner` is kept true for
/// exactly as long as this thread holds the real HID device, so the FFI
/// layer knows whether `ntd_set_mode`/`ntd_set_active_area` should write
/// `shared` directly or forward a command to whichever process does.
pub fn spawn(shared: Arc<SharedState>, is_owner: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || run_engine(&shared, &is_owner))
}

fn run_engine(shared: &Arc<SharedState>, is_owner: &Arc<AtomicBool>) {
    log::info!(target: "EngineLoop", "Starting embedded engine loop");

    loop {
        let shared_clone = Arc::clone(shared);
        let is_owner_clone = Arc::clone(is_owner);
        let result = panic::catch_unwind(move || {
            engine_iteration(&shared_clone, &is_owner_clone);
        });

        if let Err(err) = result {
            log::error!(target: "EngineLoop", "THREAD CRASHED: {err:?}");
        }

        if shared.shutdown_requested.load(Ordering::Relaxed) {
            break;
        }

        log::warn!(target: "EngineLoop", "Engine context terminated, restarting in 1 second...");
        thread::sleep(Duration::from_secs(1));
    }
}

fn engine_iteration(shared: &Arc<SharedState>, is_owner: &Arc<AtomicBool>) {
    // Held for the entire branch body, which is exactly as long as this
    // process should keep the real HID device open.
    if let Some(_hid_owner) = try_acquire_hid_owner() {
        owner_iteration(shared, is_owner);
    } else {
        reader_iteration(shared, is_owner);
    }
}

/// Resets `is_owner` to false when an owner-role stint ends, however it
/// ends (shutdown, HID error, panic unwinding through `owner_iteration`).
struct OwnerFlagGuard<'a>(&'a AtomicBool);

impl Drop for OwnerFlagGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// Applies whatever config/state the current HID owner published to this
/// `SharedState`, so `ntd_poll_state` reflects live tablet data identically
/// whether this process is the owner or a reader.
fn apply_shm_snapshot(
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

/// Maps a raw [`SdkPublicState::status`] byte back to `TabletStatus`.
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

/// How often a reader retries becoming the HID owner (e.g. the previous
/// owner exited) and how often it polls the shared segment for fresh state.
const OWNER_PROMOTION_RETRY_INTERVAL: Duration = Duration::from_secs(3);
const SHM_READER_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Runs this process as a non-owner: mirrors the current HID owner's
/// published state into `shared` instead of touching a real device, and
/// periodically retries promotion to owner.
fn reader_iteration(shared: &Arc<SharedState>, is_owner: &Arc<AtomicBool>) {
    log::info!(target: "EngineLoop", "Another process owns the HID device; running in reader mode");

    let mut reader = ShmReader::open();
    let mut last_config_version = None;
    let mut last_promotion_attempt = Instant::now();

    loop {
        if shared.shutdown_requested.load(Ordering::Relaxed) {
            return;
        }

        if Instant::now().duration_since(last_promotion_attempt) >= OWNER_PROMOTION_RETRY_INTERVAL {
            last_promotion_attempt = Instant::now();
            // Held for the rest of this function's life, same as the
            // top-level branch in `engine_iteration`.
            if let Some(_hid_owner) = try_acquire_hid_owner() {
                log::info!(target: "EngineLoop", "Promoted to HID owner, taking over the real device");
                owner_iteration(shared, is_owner);
                return;
            }
        }

        if reader.is_none() {
            reader = ShmReader::open();
        }

        if let Some(snapshot) = reader.as_ref().and_then(ShmReader::read) {
            apply_shm_snapshot(shared, &snapshot, &mut last_config_version);
        }

        thread::sleep(SHM_READER_POLL_INTERVAL);
    }
}

/// Writes a new driver mode into `shared` and bumps `config_version`. The
/// single code path both a local owner-side caller (`ntd_set_mode`) and a
/// reader's forwarded command ([`SdkCommandHandler::set_mode`]) go through.
pub fn apply_set_mode(shared: &Arc<SharedState>, mode: DriverMode) {
    let mut config = shared.config.write().unwrap_or_log("config");
    config.mode = mode;
    drop(config);
    shared.config_version.fetch_add(1, Ordering::SeqCst);
}

/// Writes a new active area into `shared` (clamped to the current device's
/// physical surface) and bumps `config_version`. The single code path both a
/// local owner-side caller (`ntd_set_active_area`) and a reader's forwarded
/// command ([`SdkCommandHandler::set_active_area`]) go through.
pub fn apply_set_active_area(shared: &Arc<SharedState>, area: ActiveArea) {
    let (phys_w, phys_h) = shared
        .device_state
        .read()
        .unwrap_or_log("device_state")
        .physical_size;

    let mut config = shared.config.write().unwrap_or_log("config");
    config.active_area = area;
    config.active_area.clamp_to_surface(phys_w, phys_h);
    drop(config);
    shared.config_version.fetch_add(1, Ordering::SeqCst);
}

/// Applies a reader's `Request` to this owner's real `SharedState`, through
/// the exact same validation/write path `ntd_set_mode`/`ntd_set_active_area`
/// use for a local (owner-side) caller.
struct SdkCommandHandler {
    shared: Arc<SharedState>,
}

impl CommandHandler for SdkCommandHandler {
    fn set_mode(&self, mode: DriverMode) {
        apply_set_mode(&self.shared, mode);
    }

    fn set_active_area(&self, area: ActiveArea) {
        apply_set_active_area(&self.shared, area);
    }
}

/// Builds an [`SdkPublicState`] snapshot from this iteration's live values
/// and publishes it, so every reader process sees this owner's tablet data.
fn publish_shm_state(
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

/// Runs this process as the HID owner: detect/drain/poll the real device,
/// write every processed frame into `shared.processed_frame`, publish it
/// into the shared segment, and answer config-write commands from readers.
fn owner_iteration(shared: &Arc<SharedState>, is_owner: &Arc<AtomicBool>) {
    is_owner.store(true, Ordering::Relaxed);
    let _owner_flag_guard = OwnerFlagGuard(is_owner);

    let hid_init_start = Instant::now();
    let hid_api = match hidapi::HidApi::new() {
        Ok(api) => {
            *shared
                .engine_status
                .write()
                .unwrap_or_reset("engine_status") =
                next_tablet_driver::engine::state::EngineStatus::Running;
            api
        }
        Err(e) => {
            log::error!(target: "EngineLoop", "CRITICAL: Failed to initialise HID API: {e}");
            *shared
                .engine_status
                .write()
                .unwrap_or_reset("engine_status") =
                next_tablet_driver::engine::state::EngineStatus::Failed(e.to_string());
            return;
        }
    };
    log::info!(target: "EngineLoop", "HID API initialised in {:.2?}", hid_init_start.elapsed());

    let mut pipeline = Pipeline::new();
    let mut local_config = shared.config.read().unwrap_or_log("config").clone();
    let mut filters = init_filter_pipeline(shared, &local_config);

    let shm_writer = ShmWriter::create();
    if shm_writer.is_none() {
        log::warn!(target: "EngineLoop", "Failed to create shared state segment; other processes won't see this instance's tablet data");
    }

    let command_handler: Arc<dyn CommandHandler> = Arc::new(SdkCommandHandler {
        shared: Arc::clone(shared),
    });
    // Kept alive for the rest of this function; dropping it stops the
    // listener thread. Only logged on failure: the HID owner lock already
    // guarantees this is the only owner, so a bind failure here "shouldn't
    // happen in practice" per `CommandListener::spawn`'s doc comment.
    let _command_listener = CommandListener::spawn(command_handler)
        .inspect_err(|e| {
            log::warn!(target: "EngineLoop", "Failed to start command listener: {e}");
        })
        .ok();

    loop {
        if shared.shutdown_requested.load(Ordering::Relaxed) {
            log::info!(target: "EngineLoop", "Shutdown requested, exiting engine loop");
            break;
        }

        if let Some((device, driver, vid, pid)) = detect_tablet(&hid_api) {
            log::info!(target: "EngineLoop", "Device connected: {vid:04x}:{pid:04x}");
            on_device_connected(shared, driver.as_ref(), vid, pid, &mut local_config);
            let mut local_config_version = shared.config_version.load(Ordering::Relaxed);

            // Drain stale packets left by the init sequence to prevent a
            // cursor teleport on the very first frame.
            let mut drain_buf = [0u8; 64];
            let drain_deadline = Instant::now() + Duration::from_millis(100);
            while Instant::now() < drain_deadline {
                if shared.shutdown_requested.load(Ordering::Relaxed) {
                    break;
                }
                match device.read_timeout(&mut drain_buf, 10) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => (),
                }
            }
            pipeline.reset_relative();

            run_polling_loop(
                &device,
                driver.as_ref(),
                shared,
                &mut pipeline,
                &mut filters,
                &mut local_config,
                &mut local_config_version,
                shm_writer.as_ref(),
            );

            if shared.shutdown_requested.load(Ordering::Relaxed) {
                log::info!(target: "EngineLoop", "Shutdown requested, exiting engine loop after polling");
                break;
            }
            log::warn!(target: "EngineLoop", "Polling loop exited, restarting...");
        }

        on_disconnected(shared);
        thread::sleep(Duration::from_millis(500));
    }
    on_disconnected(shared);
}

fn init_filter_pipeline(shared: &Arc<SharedState>, config: &MappingConfig) -> FilterPipeline {
    let mut filters = FilterPipeline::new();
    filters.add(Box::new(
        next_tablet_driver::filters::antichatter::DevocubAntichatter::new(),
    ));
    filters.add(Box::new(
        next_tablet_driver::filters::stats::SpeedStatsFilter::new(Arc::clone(shared)),
    ));
    filters.update_config(config);
    filters
}

fn on_device_connected(
    shared: &Arc<SharedState>,
    driver: &dyn next_tablet_driver::drivers::NextTabletDriver,
    vid: u16,
    pid: u16,
    local_config: &mut MappingConfig,
) {
    let size = driver.get_physical_specs();
    let (mw, mh, mp) = driver.get_specs();

    let new_device = next_tablet_driver::engine::state::DeviceState {
        name: driver.get_name().to_string(),
        vid,
        pid,
        physical_size: size,
        hardware_size: (mw, mh),
        max_pressure: mp,
    };

    *shared.device_state.write().unwrap_or_reset("device_state") = new_device.clone();
    log::info!(target: "EngineLoop", "Tablet metadata populated: {}", new_device.name);

    let mut is_first = shared.is_first_run.write().unwrap_or_reset("is_first_run");
    if *is_first {
        let mut config = shared.config.write().unwrap_or_log("config");
        config.active_area.w = size.0;
        config.active_area.h = size.1;
        config.active_area.x = size.0 / 2.0;
        config.active_area.y = size.1 / 2.0;
        *is_first = false;
        drop(is_first);
        *local_config = config.clone();
        drop(config);
        shared.config_version.fetch_add(1, Ordering::SeqCst);
    }
}

fn on_disconnected(shared: &Arc<SharedState>) {
    log::info!(target: "EngineLoop", "Device disconnected, resetting shared state");
    *shared.device_state.write().unwrap_or_reset("device_state") =
        next_tablet_driver::engine::state::DeviceState::default();
    *shared.tablet_data.write().unwrap_or_reset("tablet_data") =
        next_tablet_driver::drivers::TabletData::default();
}

/// The main packet reading loop, driving the real HID device. Adapted from
/// the desktop app's `run_polling_loop`: no OS input injection, no UI
/// channel. Every processed frame lands in `shared.processed_frame` and the
/// shared segment for `ntd_poll_state`/readers to pick up.
#[allow(clippy::too_many_arguments)]
fn run_polling_loop(
    device: &hidapi::HidDevice,
    driver: &dyn next_tablet_driver::drivers::NextTabletDriver,
    shared: &Arc<SharedState>,
    pipeline: &mut Pipeline,
    filters: &mut FilterPipeline,
    local_config: &mut MappingConfig,
    local_config_version: &mut u32,
    shm_writer: Option<&ShmWriter>,
) {
    let mut buf = [0u8; 64];
    let mut last_config_check = Instant::now();

    loop {
        if shared.shutdown_requested.load(Ordering::Relaxed) {
            log::debug!(target: "EngineLoop", "Shutdown requested, exiting polling loop");
            break;
        }

        match device.read_timeout(&mut buf, 500) {
            Ok(len) if len > 0 => {
                if let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    if let Some(slice) = buf.get(..len) {
                        process_packet(
                            slice,
                            driver,
                            shared,
                            pipeline,
                            filters,
                            local_config,
                            shm_writer,
                        );
                    }
                    maybe_reload_config(
                        shared,
                        filters,
                        local_config,
                        local_config_version,
                        &mut last_config_check,
                    );
                })) {
                    log::error!(target: "EngineLoop", "Packet processing panicked: {e:?}");
                }
            }
            Ok(_) => {
                // Out of range event.
                let out = TabletData {
                    status: TabletStatus::OutOfRange,
                    ..Default::default()
                };
                let frame = pipeline.process(&out, driver, local_config, filters, shared);
                *shared
                    .processed_frame
                    .write()
                    .unwrap_or_reset("processed_frame") = frame;
                *shared.tablet_data.write().unwrap_or_reset("tablet_data") = out.clone();
                if let Some(writer) = shm_writer {
                    publish_shm_state(writer, shared, &out, local_config, &frame);
                }

                maybe_reload_config(
                    shared,
                    filters,
                    local_config,
                    local_config_version,
                    &mut last_config_check,
                );
            }
            Err(e) => {
                log::error!(target: "EngineLoop", "HID read error: {e}");
                return;
            }
        }
    }
}

fn process_packet(
    raw: &[u8],
    driver: &dyn next_tablet_driver::drivers::NextTabletDriver,
    shared: &Arc<SharedState>,
    pipeline: &mut Pipeline,
    filters: &mut FilterPipeline,
    local_config: &MappingConfig,
    shm_writer: Option<&ShmWriter>,
) {
    if let Some(data) = driver.parse(raw) {
        let frame = pipeline.process(&data, driver, local_config, filters, shared);
        *shared
            .processed_frame
            .write()
            .unwrap_or_reset("processed_frame") = frame;
        *shared.tablet_data.write().unwrap_or_reset("tablet_data") = data.clone();
        if let Some(writer) = shm_writer {
            publish_shm_state(writer, shared, &data, local_config, &frame);
        }

        shared.packet_count.fetch_add(1, Ordering::Relaxed);
    }
}

fn maybe_reload_config(
    shared: &Arc<SharedState>,
    filters: &mut FilterPipeline,
    local_config: &mut MappingConfig,
    local_config_version: &mut u32,
    last_check: &mut Instant,
) {
    if Instant::now().duration_since(*last_check) < Duration::from_millis(50) {
        return;
    }
    *last_check = Instant::now();

    let cv = shared.config_version.load(Ordering::Relaxed);
    if cv != *local_config_version {
        let config = shared.config.read().unwrap_or_log("config");
        *local_config = config.clone();
        drop(config);
        *local_config_version = cv;
        filters.update_config(local_config);
        log::info!(target: "EngineLoop", "Configuration reloaded to version {cv}");
    }
}
