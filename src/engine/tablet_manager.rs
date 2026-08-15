//! # Tablet Device Manager
//!
//! This module is the execution environment for the background USB polling thread.
//! It handles detecting devices, reading raw USB packets, checking for configuration
//! updates, and feeding data to the UI thread and [`Pipeline`].
//!
//! # Architecture
//!
//! ```text
//! run_manager()
//!   └── manager_thread_iteration()
//!         ├── owner_iteration()      (this process holds the HID owner lock)
//!         │    ├── init_thread_priority()
//!         │    ├── init_filter_pipeline()
//!         │    └── loop
//!         │          ├── on_device_connected()
//!         │          ├── run_polling_loop()
//!         │          │    ├── process_packet()      (publishes shm state)
//!         │          │    └── maybe_reload_config()
//!         │          └── on_disconnected()
//!         └── reader_iteration()     (another process owns the HID device)
//!              └── loop
//!                    ├── apply_shm_snapshot()
//!                    └── try_acquire_hid_owner()      (periodic promotion retry)
//! ```
//!
//! See `engine::interop` for the HID-owner arbitration mechanism: exactly one
//! process (this desktop app, or an SDK-embedded game) opens the real HID
//! device at a time, and every other process mirrors its state instead.

use crate::core::config::models::{ActiveArea, DriverMode, MappingConfig};
use crate::drivers::{TabletData, TabletStatus, detect_tablet};
use crate::engine::injector::Injector;
use crate::engine::interop::command::{CommandHandler, CommandListener};
use crate::engine::interop::lock::try_acquire_hid_owner;
use crate::engine::interop::shm::{DEVICE_NAME_CAPACITY, SdkPublicState, ShmReader, ShmWriter};
use crate::engine::pipeline::{Pipeline, ProcessedFrame};
use crate::engine::state::{LockRecoveryExt, SharedState, WriteRecoverExt};
use crate::filters::FilterPipeline;
use crossbeam_channel::Sender;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

/// Starts the background USB polling loop.
pub fn run_manager(shared: &Arc<SharedState>, tablet_sender: &Sender<TabletData>) {
    log::info!(target: "TabletManager", "Starting device manager thread");

    loop {
        let shared_clone = Arc::clone(shared);
        let sender_clone = tablet_sender.clone();

        let result = panic::catch_unwind(move || {
            manager_thread_iteration(&shared_clone, &sender_clone);
        });

        if let Err(err) = result {
            log::error!(target: "TabletManager", "THREAD CRASHED: {err:?}");
        }

        if shared.shutdown_requested.load(Ordering::Relaxed) {
            break;
        }

        log::warn!(target: "TabletManager", "Engine context terminated, restarting in 1 second...");
        thread::sleep(Duration::from_secs(1));
    }
}

fn manager_thread_iteration(shared_clone: &Arc<SharedState>, sender_clone: &Sender<TabletData>) {
    // The binding below is held for the entire branch body, which is exactly
    // as long as this process should keep the real HID device open.
    if let Some(_hid_owner) = try_acquire_hid_owner() {
        owner_iteration(shared_clone, sender_clone);
    } else {
        reader_iteration(shared_clone, sender_clone);
    }
}

/// Applies whatever config/state a remote HID owner published to this
/// process's own `SharedState`, so the desktop UI reflects live tablet data
/// even when another process (another SDK-embedded game, or a second
/// desktop instance) is the one actually driving the device.
///
/// Local config writes made through the desktop UI while in reader mode are
/// intentionally out of scope here. The desktop UI has no notion yet of
/// "control the remote owner's device" versus "edit my own settings"; that
/// distinction belongs to a UI-level change, not this wiring.
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

/// How often a reader retries becoming the HID owner (e.g. the previous
/// owner exited) and how often it polls the shared segment for fresh state.
const OWNER_PROMOTION_RETRY_INTERVAL: Duration = Duration::from_secs(3);
const SHM_READER_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Runs this process as a non-owner: mirrors the current HID owner's
/// published state into the local `SharedState` instead of touching a real
/// device, and periodically retries promotion to owner.
fn reader_iteration(shared: &Arc<SharedState>, sender: &Sender<TabletData>) {
    log::info!(target: "TabletManager", "Another process owns the HID device; running in reader mode");

    let mut reader = ShmReader::open();
    let mut last_config_version = None;
    let mut last_promotion_attempt = Instant::now();

    loop {
        if shared.shutdown_requested.load(Ordering::Relaxed) {
            return;
        }
        if shared.reload_requested.swap(false, Ordering::Relaxed) {
            return;
        }

        if Instant::now().duration_since(last_promotion_attempt) >= OWNER_PROMOTION_RETRY_INTERVAL {
            last_promotion_attempt = Instant::now();
            // Held for the rest of this function's life, same as the
            // top-level branch in `manager_thread_iteration`.
            if let Some(_hid_owner) = try_acquire_hid_owner() {
                log::info!(target: "TabletManager", "Promoted to HID owner, taking over the real device");
                owner_iteration(shared, sender);
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

/// Applies a reader's [`Request`](crate::engine::interop::command::Request)
/// to this owner's real `SharedState`, through the exact same
/// validation/write path a local caller (the desktop UI) would use.
struct DesktopCommandHandler {
    shared: Arc<SharedState>,
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

/// Runs this process as the HID owner: the pre-existing detect/poll/process
/// loop, plus publishing live state into the shared segment and listening
/// for config-write commands from readers.
fn owner_iteration(shared_clone: &Arc<SharedState>, sender_clone: &Sender<TabletData>) {
    let hid_init_start = Instant::now();
    let hid_api = match hidapi::HidApi::new() {
        Ok(api) => {
            *shared_clone
                .engine_status
                .write()
                .unwrap_or_reset("engine_status") = crate::engine::state::EngineStatus::Running;
            api
        }
        Err(e) => {
            log::error!(target: "HID", "CRITICAL: Failed to initialise HID API: {e}");
            let error_str = e.to_string();
            crate::app::telemetry::capture_event(
                "engine_error",
                Some(serde_json::json!({
                    "error_message": error_str,
                    "context": "HID API Initialization"
                })),
            );
            *shared_clone
                .engine_status
                .write()
                .unwrap_or_reset("engine_status") =
                crate::engine::state::EngineStatus::Failed(error_str);
            return;
        }
    };
    log::info!(target: "HID", "HID API initialised in {:.2?}", hid_init_start.elapsed());

    let mut injector = Injector::new();
    let mut pipeline = Pipeline::new();

    init_thread_priority();

    let mut local_config = shared_clone.config.read().unwrap_or_log("config").clone();
    let mut filters = init_filter_pipeline(shared_clone, &local_config);

    let shm_writer = ShmWriter::create();
    if shm_writer.is_none() {
        log::warn!(target: "TabletManager", "Failed to create shared state segment; other processes won't see this instance's tablet data");
    }

    let command_handler: Arc<dyn CommandHandler> = Arc::new(DesktopCommandHandler {
        shared: Arc::clone(shared_clone),
    });
    // Kept alive for the rest of this function; dropping it stops the
    // listener thread. Only logged on failure: "shouldn't happen in
    // practice" per `CommandListener::spawn`'s doc comment, since the HID
    // owner lock already guarantees this is the only owner.
    let _command_listener = CommandListener::spawn(command_handler)
        .inspect_err(|e| {
            log::warn!(target: "TabletManager", "Failed to start command listener: {e}");
        })
        .ok();

    loop {
        if shared_clone.shutdown_requested.load(Ordering::Relaxed) {
            log::info!(target: "TabletManager", "Shutdown requested, exiting manager loop");
            break;
        }

        if shared_clone.reload_requested.swap(false, Ordering::Relaxed) {
            log::warn!(target: "TabletManager", "Engine reload requested, tearing down context...");
            break;
        }

        if let Some((device, driver, vid, pid)) = detect_tablet(&hid_api) {
            log::info!(target: "HID", "Device connected: {vid:04x}:{pid:04x}");
            on_device_connected(shared_clone, driver.as_ref(), vid, pid, &mut local_config);
            let mut local_config_version = shared_clone.config_version.load(Ordering::Relaxed);

            // Drain stale packets left by init sequence to prevent cursor teleport
            let mut drain_buf = [0u8; 64];
            let drain_deadline = Instant::now() + Duration::from_millis(100);
            while Instant::now() < drain_deadline {
                if shared_clone.shutdown_requested.load(Ordering::Relaxed)
                    || shared_clone.reload_requested.load(Ordering::Relaxed)
                {
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
                shared_clone,
                sender_clone,
                &mut pipeline,
                &mut injector,
                &mut filters,
                &mut local_config,
                &mut local_config_version,
                shm_writer.as_ref(),
            );

            if shared_clone.shutdown_requested.load(Ordering::Relaxed) {
                log::info!(target: "TabletManager", "Shutdown requested, exiting manager loop after polling");
                break;
            }
            if shared_clone.reload_requested.load(Ordering::Relaxed) {
                log::warn!(target: "TabletManager", "Reload requested, breaking out to restart context...");
                break;
            }
            log::warn!(target: "HID", "Polling loop exited, restarting...");
        }

        on_disconnected(shared_clone);
        thread::sleep(Duration::from_millis(500));
    }
    on_disconnected(shared_clone);
}

/// Adjusts the execution priority of the polling thread.
///
/// On Windows, sets the thread priority to `TIME_CRITICAL`.
/// On Linux, attempts to nice the thread to `-11` (requires `CAP_SYS_NICE` or root permissions).
fn init_thread_priority() {
    // SAFETY: These are standard OS-specific calls to increase thread priority
    // for low-latency USB polling.
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
        };
        // SAFETY: Retrieving the handle to the current thread.
        let thread = unsafe { GetCurrentThread() };
        // SAFETY: Increasing thread priority to TIME_CRITICAL for low-latency USB polling.
        if unsafe { SetThreadPriority(thread, THREAD_PRIORITY_TIME_CRITICAL) } == 0 {
            log::warn!(target: "TabletManager", "Failed to set thread priority to TIME_CRITICAL");
        } else {
            log::info!(target: "TabletManager", "Thread priority set to TIME_CRITICAL");
        }
    }
    #[cfg(target_os = "linux")]
    // SAFETY: calling `libc::nice` is safe to change the current thread priority.
    // The return value of -1 is checked to gracefully handle permission failures.
    unsafe {
        if libc::nice(-11) == -1 {
            log::info!(target: "TabletManager", "Running at normal priority (CAP_SYS_NICE not available)");
        } else {
            log::info!(target: "TabletManager", "Thread priority increased (nice -11)");
        }
    }
}

/// Instantiates and configures the signal filtering pipeline.
///
/// Registers standard pipeline stages such as `DevocubAntichatter` and `SpeedStatsFilter`.
fn init_filter_pipeline(shared: &Arc<SharedState>, config: &MappingConfig) -> FilterPipeline {
    let mut filters = FilterPipeline::new();
    filters.add(Box::new(
        crate::filters::antichatter::DevocubAntichatter::new(),
    ));
    filters.add(Box::new(crate::filters::stats::SpeedStatsFilter::new(
        Arc::clone(shared),
    )));
    filters.update_config(config);
    filters
}

/// Handles tablet connection events.
///
/// Populates shared state device metadata, resets configuration parameters, and triggers
/// a configuration increment if this is the first run to propagate the initial active area settings.
fn on_device_connected(
    shared: &Arc<SharedState>,
    driver: &dyn crate::drivers::NextTabletDriver,
    vid: u16,
    pid: u16,
    local_config: &mut MappingConfig,
) {
    let size = driver.get_physical_specs();
    let (mw, mh, _) = driver.get_specs();

    let new_device = crate::engine::state::DeviceState {
        name: driver.get_name().to_string(),
        vid,
        pid,
        physical_size: size,
        hardware_size: (mw, mh),
    };

    *shared.device_state.write().unwrap_or_reset("device_state") = new_device.clone();
    log::info!(target: "TabletManager", "Tablet metadata populated: {}", new_device.name);

    crate::app::telemetry::capture_event_dedup(
        "tablet_connected",
        Some(serde_json::json!({
            "tablet_model": new_device.name,
            "vendor_id": format!("{:#06X}", new_device.vid),
            "product_id": format!("{:#06X}", new_device.pid),
        })),
        Some(serde_json::json!({
            "tablets_owned": [&new_device.name],
            "last_tablet_connected": &new_device.name,
        })),
        &new_device.name,
    );

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

/// Handles tablet disconnection events.
///
/// Cleans up shared state, restoring the default "No Tablet Detected" device state and zeroing
/// coordinate parameters to prevent incorrect pointer inputs.
fn on_disconnected(shared: &Arc<SharedState>) {
    log::info!(target: "HID", "Device disconnected, resetting shared state");
    *shared.device_state.write().unwrap_or_reset("device_state") =
        crate::engine::state::DeviceState::default();
    *shared.tablet_data.write().unwrap_or_reset("tablet_data") =
        crate::drivers::TabletData::default();
}

/// The main packet reading loop of the engine thread.
///
/// Polls the raw HID device for byte reports and coordinates configuration reloading and packet
/// processing.
#[allow(clippy::too_many_arguments)]
fn run_polling_loop(
    device: &hidapi::HidDevice,
    driver: &dyn crate::drivers::NextTabletDriver,
    shared: &Arc<SharedState>,
    tablet_sender: &Sender<TabletData>,
    pipeline: &mut Pipeline,
    injector: &mut Injector,
    filters: &mut FilterPipeline,
    local_config: &mut MappingConfig,
    local_config_version: &mut u32,
    shm_writer: Option<&ShmWriter>,
) {
    let mut buf = [0u8; 64];
    let mut last_config_check = Instant::now();
    let mut last_stats_update = Instant::now();
    let mut last_packet_time: Option<(Instant, crate::drivers::TabletStatus)> = None;

    loop {
        if shared.shutdown_requested.load(Ordering::Relaxed) {
            log::debug!(target: "TabletManager", "Shutdown requested, exiting polling loop");
            break;
        }

        if shared.reload_requested.load(Ordering::Relaxed) {
            log::debug!(target: "TabletManager", "Reload requested, exiting polling loop");
            break;
        }

        let read_start = Instant::now();
        match device.read_timeout(&mut buf, 500) {
            // Reduced from 1000 to 500 for faster shutdown check
            Ok(len) if len > 0 => {
                let read_duration = read_start.elapsed();
                if let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    if let Some(slice) = buf.get(..len) {
                        process_packet(
                            slice,
                            read_start,
                            read_duration,
                            driver,
                            shared,
                            tablet_sender,
                            pipeline,
                            injector,
                            filters,
                            local_config,
                            &mut last_stats_update,
                            &mut last_packet_time,
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
                    log::error!(target: "TabletManager", "Packet processing panicked: {e:?}");
                }
            }
            Ok(_) => {
                // Out of range event
                let out = TabletData {
                    status: crate::drivers::TabletStatus::OutOfRange,
                    ..Default::default()
                };
                let frame = pipeline.process(&out, driver, local_config, filters, shared);
                inject_frame(injector, &out, local_config, &frame);
                *shared
                    .processed_frame
                    .write()
                    .unwrap_or_reset("processed_frame") = frame;
                if let Some(writer) = shm_writer {
                    publish_shm_state(writer, shared, &out, local_config, &frame);
                }
                if shared.is_visible.load(Ordering::Relaxed) {
                    let _ = tablet_sender.try_send(out);
                }

                // Still check for config even when out of range
                maybe_reload_config(
                    shared,
                    filters,
                    local_config,
                    local_config_version,
                    &mut last_config_check,
                );
            }
            Err(e) => {
                log::error!(target: "HID", "HID read error: {e}");
                return;
            }
        }
    }
}

/// Drives OS input injection from a processed frame.
///
/// Mirrors the branching that used to live inside `Pipeline::process()` itself:
/// disconnected pens release the button and drop proximity, non-positional
/// reports (aux/tool-ID) only drop proximity, and positional reports move the
/// cursor (absolute or relative, per the active driver mode) before syncing
/// the button state. Kept here rather than in the pipeline so that `Pipeline`
/// never touches the OS, which is required for the embedded SDK use case.
fn inject_frame(
    injector: &mut Injector,
    data: &TabletData,
    config: &MappingConfig,
    frame: &ProcessedFrame,
) {
    if !data.is_connected {
        injector.set_left_button(false);
        injector.set_proximity(false);
        return;
    }

    if !matches!(
        data.status,
        crate::drivers::TabletStatus::Contact
            | crate::drivers::TabletStatus::Hover
            | crate::drivers::TabletStatus::Active
    ) {
        injector.set_proximity(false);
        return;
    }

    match config.mode {
        DriverMode::Absolute => {
            injector.move_absolute(
                frame.screen_x,
                frame.screen_y,
                frame.u,
                frame.v,
                frame.pressure,
                frame.tilt_x,
                frame.tilt_y,
            );
        }
        DriverMode::Relative => {
            injector.move_relative(frame.screen_x, frame.screen_y);
        }
    }

    injector.set_left_button(frame.is_down);
}

/// Parses, processes, and submits a raw USB packet.
///
/// Evaluates parser and filter execution durations and reports performance lag spikes to
/// the logs, when parsing and processing time exceeds 5.0ms, or when the duration between
/// consecutive active reports exceeds 25.0ms.
///
/// Emits statistics updates and forwards output frames to the GUI thread.
#[allow(clippy::too_many_arguments)]
fn process_packet(
    raw: &[u8],
    read_start: Instant,
    read_duration: Duration,
    driver: &dyn crate::drivers::NextTabletDriver,
    shared: &Arc<SharedState>,
    tablet_sender: &Sender<TabletData>,
    pipeline: &mut Pipeline,
    injector: &mut Injector,
    filters: &mut FilterPipeline,
    local_config: &MappingConfig,
    last_stats_update: &mut Instant,
    last_packet_time: &mut Option<(Instant, crate::drivers::TabletStatus)>,
    shm_writer: Option<&ShmWriter>,
) {
    let parse_start = Instant::now();
    if let Some(mut data) = driver.parse(raw) {
        let parse_duration = parse_start.elapsed();
        data.receive_time = Some(read_start);
        data.parser_time = parse_duration;

        let process_start = Instant::now();
        let frame = pipeline.process(&data, driver, local_config, filters, shared);
        inject_frame(injector, &data, local_config, &frame);
        *shared
            .processed_frame
            .write()
            .unwrap_or_reset("processed_frame") = frame;
        if let Some(writer) = shm_writer {
            publish_shm_state(writer, shared, &data, local_config, &frame);
        }
        let process_duration = process_start.elapsed();

        let total_dur = parse_duration + process_duration;
        if total_dur > Duration::from_millis(5) {
            log::warn!(
                target: "PerfSpike",
                "LAG SPIKE: Packet parsing & processing took {total_dur:.2?} (parsing: {parse_duration:.2?}, processing: {process_duration:.2?}, HID read: {read_duration:.2?})"
            );
        }

        let now = Instant::now();
        if let Some((last_time, last_status)) = last_packet_time {
            let is_curr_active = !matches!(
                data.status,
                crate::drivers::TabletStatus::Disconnected
                    | crate::drivers::TabletStatus::OutOfRange
            );
            let is_prev_active = !matches!(
                last_status,
                crate::drivers::TabletStatus::Disconnected
                    | crate::drivers::TabletStatus::OutOfRange
            );
            if is_curr_active && is_prev_active {
                let interval = now.duration_since(*last_time);
                if interval > Duration::from_millis(25) {
                    log::warn!(
                        target: "PerfSpike",
                        "LAG SPIKE: Delay between active packets was {interval:.2?} (exceeded 25ms threshold)"
                    );
                }
            }
        }
        *last_packet_time = Some((now, data.status));

        shared.packet_count.fetch_add(1, Ordering::Relaxed);

        // Update statistics (throttled to ~60Hz)
        let now = Instant::now();
        if now.duration_since(*last_stats_update) > Duration::from_millis(16)
            && let Ok(mut stats) = shared.stats.write()
        {
            *last_stats_update = now;
            stats.total_packets = u64::from(shared.packet_count.load(Ordering::Relaxed));

            let hr_ms = read_duration.as_secs_f32() * 1000.0;
            stats.hid_read_ms = hr_ms;
            stats.min_hid_read_ms = stats.min_hid_read_ms.min(hr_ms);
            stats.max_hid_read_ms = stats.max_hid_read_ms.max(hr_ms);
            stats.avg_hid_read_ms =
                (hr_ms - stats.avg_hid_read_ms).mul_add(0.05, stats.avg_hid_read_ms);

            let p_ms = parse_duration.as_secs_f32() * 1000.0;
            stats.parser_ms = p_ms;
            stats.min_parser_ms = stats.min_parser_ms.min(p_ms);
            stats.max_parser_ms = stats.max_parser_ms.max(p_ms);
            stats.avg_parser_ms = (p_ms - stats.avg_parser_ms).mul_add(0.05, stats.avg_parser_ms);
        }

        // Only send to the UI channel when the window is visible.
        // When hidden in the system tray, the UI thread is idle and
        // nobody consumes the channel - skipping prevents unbounded growth.
        if shared.is_visible.load(Ordering::Relaxed) {
            let _ = tablet_sender.try_send(data);
        }
    }
}

/// Checks for changed configuration versions and applies hot-reloading to the pipelines.
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
        log::info!(target: "Config", "Configuration reloaded to version {cv}");
        crate::settings::log_mapping_config(local_config, &format!("Reload v{cv}"));
    }
}
