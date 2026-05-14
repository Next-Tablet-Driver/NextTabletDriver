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
use eframe::egui;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

/// Starts the background USB polling loop.
pub fn run_manager(
    shared: Arc<SharedState>,
    _ctx: egui::Context,
    tablet_sender: Sender<TabletData>,
) {
    log::info!(target: "TabletManager", "Starting device manager thread");

    let result = panic::catch_unwind(move || {
        let hid_init_start = Instant::now();
        let hid_api = match hidapi::HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                log::error!(target: "HID", "CRITICAL: Failed to initialise HID API: {e}");
                return;
            }
        };
        log::info!(target: "HID", "HID API initialised in {:.2?}", hid_init_start.elapsed());

        let mut injector = Injector::new();
        let mut pipeline = Pipeline::new();

        init_thread_priority();

        let mut local_config = shared.config.read().unwrap_or_log("config").clone();
        let mut filters = init_filter_pipeline(&shared, &local_config);

        loop {
            if shared.shutdown_requested.load(Ordering::Relaxed) {
                log::info!(target: "TabletManager", "Shutdown requested, exiting manager loop");
                break;
            }

            if let Some((device, driver, vid, pid)) = detect_tablet(&hid_api) {
                log::info!(target: "HID", "Device connected: {vid:04x}:{pid:04x}");
                on_device_connected(&shared, driver.as_ref(), vid, pid, &mut local_config);
                let mut local_config_version = shared.config_version.load(Ordering::Relaxed);

                // Drain stale packets left by init sequence to prevent cursor teleport
                let mut drain_buf = [0u8; 64];
                let drain_deadline = Instant::now() + Duration::from_millis(100);
                while Instant::now() < drain_deadline {
                    if shared.shutdown_requested.load(Ordering::Relaxed) { break; }
                    match device.read_timeout(&mut drain_buf, 10) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => continue,
                    }
                }
                pipeline.reset_relative();

                run_polling_loop(
                    &device,
                    driver.as_ref(),
                    &shared,
                    &tablet_sender,
                    &mut pipeline,
                    &mut injector,
                    &mut filters,
                    &mut local_config,
                    &mut local_config_version,
                );

                if shared.shutdown_requested.load(Ordering::Relaxed) {
                    log::info!(target: "TabletManager", "Shutdown requested, exiting manager loop after polling");
                    break;
                }
                log::warn!(target: "HID", "Polling loop exited, restarting...");
            }

            on_disconnected(&shared);
            thread::sleep(Duration::from_millis(500));
        }
    });

    if let Err(err) = result {
        log::error!(target: "TabletManager", "THREAD CRASHED: {err:?}");
    }
}

fn init_thread_priority() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
        };
        if SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) == 0 {
            log::warn!(target: "TabletManager", "Failed to set thread priority to TIME_CRITICAL");
        } else {
            log::info!(target: "TabletManager", "Thread priority set to TIME_CRITICAL");
        }
    }
    #[cfg(target_os = "linux")]
    unsafe {
        if libc::nice(-11) == -1 {
            log::info!(target: "TabletManager", "Running at normal priority (CAP_SYS_NICE not available)");
        } else {
            log::info!(target: "TabletManager", "Thread priority increased (nice -11)");
        }
    }
}

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
    log::info!(target: "HID", "Device disconnected, resetting shared state");
    *shared.device_state.write().unwrap_or_reset("device_state") = crate::engine::state::DeviceState::default();
    *shared.tablet_data.write().unwrap_or_reset("tablet_data") = crate::drivers::TabletData::default();
}

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

    loop {
        if shared.shutdown_requested.load(Ordering::Relaxed) {
            log::debug!(target: "TabletManager", "Shutdown requested, exiting polling loop");
            break;
        }

        let read_start = Instant::now();
        match device.read_timeout(&mut buf, 500) { // Reduced from 1000 to 500 for faster shutdown check
            Ok(len) if len > 0 => {
                let read_duration = read_start.elapsed();
                if let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    process_packet(
                        &buf[..len],
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
                    );
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
                    status: "Out of Range".to_string(),
                    ..Default::default()
                };
                pipeline.process(&out, driver, local_config, injector, filters, shared);
                let _ = tablet_sender.send(out);

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
) {
    let parse_start = Instant::now();
    if let Some(mut data) = driver.parse(raw) {
        let parse_duration = parse_start.elapsed();
        data.receive_time = Some(read_start);
        data.parser_time = parse_duration;

        pipeline.process(&data, driver, local_config, injector, filters, shared);

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
            stats.avg_hid_read_ms += (hr_ms - stats.avg_hid_read_ms) * 0.05;

            let p_ms = parse_duration.as_secs_f32() * 1000.0;
            stats.parser_ms = p_ms;
            stats.min_parser_ms = stats.min_parser_ms.min(p_ms);
            stats.max_parser_ms = stats.max_parser_ms.max(p_ms);
            stats.avg_parser_ms += (p_ms - stats.avg_parser_ms) * 0.05;
        }

        let _ = tablet_sender.send(data);
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
        *local_config = shared.config.read().unwrap_or_log("config").clone();
        *local_config_version = cv;
        filters.update_config(local_config);
        log::info!(target: "Config", "Configuration reloaded to version {cv}");
    }
}
