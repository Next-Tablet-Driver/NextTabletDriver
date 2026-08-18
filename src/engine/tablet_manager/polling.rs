//! The raw HID packet read/process loop that runs while this process owns
//! the tablet device.

use super::sdk_bridge::publish_shm_state;
use crate::core::config::models::{DriverMode, MappingConfig};
use crate::drivers::{TabletData, TabletStatus};
use crate::engine::injector::Injector;
use crate::engine::interop::shm::ShmWriter;
use crate::engine::pipeline::{Pipeline, ProcessedFrame};
use crate::engine::state::{LockRecoveryExt, SharedState, WriteRecoverExt};
use crate::filters::FilterPipeline;
use crossbeam_channel::Sender;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// The main packet reading loop of the engine thread.
///
/// Polls the raw HID device for byte reports and coordinates configuration reloading and packet
/// processing.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_polling_loop(
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
    let mut last_packet_time: Option<(Instant, TabletStatus)> = None;

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
                    status: TabletStatus::OutOfRange,
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
        TabletStatus::Contact | TabletStatus::Hover | TabletStatus::Active
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
    last_packet_time: &mut Option<(Instant, TabletStatus)>,
    shm_writer: Option<&ShmWriter>,
) {
    let parse_start = Instant::now();
    if let Some(mut data) = driver.parse(raw) {
        let parse_duration = parse_start.elapsed();
        data.receive_time = Some(read_start);
        data.parser_time = parse_duration;

        let process_start = Instant::now();
        let frame = pipeline.process(&data, driver, local_config, filters, shared);

        let inject_start = Instant::now();
        inject_frame(injector, &data, local_config, &frame);
        let inject_duration = inject_start.elapsed();

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
                "LAG SPIKE: Packet parsing & processing took {total_dur:.2?} (parsing: {parse_duration:.2?}, processing: {process_duration:.2?}, inject: {inject_duration:.2?}, HID read: {read_duration:.2?})"
            );
        }

        let now = Instant::now();
        if let Some((last_time, last_status)) = last_packet_time {
            let is_curr_active = !matches!(
                data.status,
                TabletStatus::Disconnected | TabletStatus::OutOfRange
            );
            let is_prev_active = !matches!(
                last_status,
                TabletStatus::Disconnected | TabletStatus::OutOfRange
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

            let i_ms = inject_duration.as_secs_f32() * 1000.0;
            stats.inject_ms = i_ms;
            stats.min_inject_ms = stats.min_inject_ms.min(i_ms);
            stats.max_inject_ms = stats.max_inject_ms.max(i_ms);
            stats.avg_inject_ms = (i_ms - stats.avg_inject_ms).mul_add(0.05, stats.avg_inject_ms);
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
