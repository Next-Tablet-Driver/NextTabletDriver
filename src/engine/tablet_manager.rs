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
//!   ├── init_thread_priority()
//!   ├── init_filter_pipeline()
//!   └── loop
//!         ├── on_device_connected()
//!         ├── run_polling_loop()
//!         │    ├── process_packet()
//!         │    └── maybe_reload_config()
//!         └── on_disconnected()
//! ```

use crate::core::config::models::MappingConfig;
use crate::drivers::{TabletData, detect_tablet};
use crate::engine::injector::Injector;
use crate::engine::pipeline::Pipeline;
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
                pipeline.process(&out, driver, local_config, injector, filters, shared);
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

/// Parses, processes, and submits a raw USB packet.
///
/// Evaluates parser and filter execution durations and reports performance lag spikes to the logs:
/// 1. If parsing + processing time exceeds 5.0ms.
/// 2. If the duration between consecutive active reports exceeds 25.0ms.
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
) {
    let parse_start = Instant::now();
    if let Some(mut data) = driver.parse(raw) {
        let parse_duration = parse_start.elapsed();
        data.receive_time = Some(read_start);
        data.parser_time = parse_duration;

        let process_start = Instant::now();
        pipeline.process(&data, driver, local_config, injector, filters, shared);
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
