//! Runs this process as the HID owner: detects and polls the real device,
//! publishes live state into the shared segment, and listens for
//! config-write commands from reader processes.

use super::polling::run_polling_loop;
use super::sdk_bridge::DesktopCommandHandler;
use crate::core::config::models::MappingConfig;
use crate::drivers::{TabletData, detect_tablet};
use crate::engine::injector::Injector;
use crate::engine::interop::command::{CommandHandler, CommandListener};
use crate::engine::interop::shm::ShmWriter;
use crate::engine::pipeline::Pipeline;
use crate::engine::state::{LockRecoveryExt, SharedState, WriteRecoverExt};
use crate::filters::FilterPipeline;
use crossbeam_channel::Sender;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

/// Runs this process as the HID owner: the pre-existing detect/poll/process
/// loop, plus publishing live state into the shared segment and listening
/// for config-write commands from readers.
pub(super) fn owner_iteration(shared_clone: &Arc<SharedState>, sender_clone: &Sender<TabletData>) {
    let hid_init_start = Instant::now();
    let hid_api = match hidapi::HidApi::new() {
        Ok(api) => {
            *shared_clone
                .lifecycle
                .engine_status
                .write()
                .unwrap_or_reset("engine_status") = crate::engine::state::EngineStatus::Running;
            api
        }
        Err(e) => {
            log::error!(target: "HID", "CRITICAL: Failed to initialise HID API: {e}");
            let error_str = e.to_string();
            crate::application::telemetry::capture_event(
                "engine_error",
                Some(serde_json::json!({
                    "error_message": error_str,
                    "context": "HID API Initialization"
                })),
            );
            *shared_clone
                .lifecycle
                .engine_status
                .write()
                .unwrap_or_reset("engine_status") =
                crate::engine::state::EngineStatus::Failed(error_str);
            return;
        }
    };
    log::info!(target: "HID", "HID API initialised in {:.2?}", hid_init_start.elapsed());

    let mut injector = match Injector::try_new() {
        Ok(injector) => injector,
        Err(e) => {
            log::error!(target: "Injector", "CRITICAL: Failed to initialise input injector: {e}");
            crate::application::telemetry::capture_event(
                "engine_error",
                Some(serde_json::json!({
                    "error_message": e,
                    "context": "Injector Initialization"
                })),
            );
            *shared_clone
                .lifecycle
                .engine_status
                .write()
                .unwrap_or_reset("engine_status") = crate::engine::state::EngineStatus::Failed(e);
            return;
        }
    };
    let mut pipeline = Pipeline::new();

    init_thread_priority();

    let mut local_config = shared_clone
        .config
        .mapping
        .read()
        .unwrap_or_log("config")
        .clone();
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
        if shared_clone
            .lifecycle
            .shutdown_requested
            .load(Ordering::Relaxed)
        {
            log::info!(target: "TabletManager", "Shutdown requested, exiting manager loop");
            break;
        }

        if shared_clone
            .config
            .reload_requested
            .swap(false, Ordering::Relaxed)
        {
            log::warn!(target: "TabletManager", "Engine reload requested, tearing down context...");
            break;
        }

        if let Some((device, driver, vid, pid)) = detect_tablet(&hid_api) {
            log::info!(target: "HID", "Device connected: {vid:04x}:{pid:04x}");
            on_device_connected(shared_clone, driver.as_ref(), vid, pid, &mut local_config);
            let mut local_config_version = shared_clone.config.version.load(Ordering::Relaxed);

            // Drain stale packets left by init sequence to prevent cursor teleport
            let mut drain_buf = [0u8; 64];
            let drain_deadline = Instant::now() + Duration::from_millis(100);
            while Instant::now() < drain_deadline {
                if shared_clone
                    .lifecycle
                    .shutdown_requested
                    .load(Ordering::Relaxed)
                    || shared_clone.config.reload_requested.load(Ordering::Relaxed)
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

            if shared_clone
                .lifecycle
                .shutdown_requested
                .load(Ordering::Relaxed)
            {
                log::info!(target: "TabletManager", "Shutdown requested, exiting manager loop after polling");
                break;
            }
            if shared_clone.config.reload_requested.load(Ordering::Relaxed) {
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
    shared.plugins.create_pipeline(config)
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
    let (mw, mh, mp) = driver.get_specs();

    let new_device = crate::engine::state::DeviceState {
        name: driver.get_name().to_string(),
        vid,
        pid,
        physical_size: size,
        hardware_size: (mw, mh),
        max_pressure: mp,
    };

    *shared.device.write().unwrap_or_reset("device") = new_device.clone();
    log::info!(target: "TabletManager", "Tablet metadata populated: {}", new_device.name);

    crate::application::telemetry::capture_event_dedup(
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

    let mut is_first = shared
        .lifecycle
        .is_first_run
        .write()
        .unwrap_or_reset("is_first_run");
    if *is_first {
        let mut config = shared.config.mapping.write().unwrap_or_log("config");
        config.active_area.w = size.0;
        config.active_area.h = size.1;
        config.active_area.x = size.0 / 2.0;
        config.active_area.y = size.1 / 2.0;
        *is_first = false;
        drop(is_first);
        *local_config = config.clone();
        drop(config);
        shared.config.version.fetch_add(1, Ordering::SeqCst);
    }
}

/// Handles tablet disconnection events.
///
/// Cleans up shared state, restoring the default "No Tablet Detected" device state and zeroing
/// coordinate parameters to prevent incorrect pointer inputs.
fn on_disconnected(shared: &Arc<SharedState>) {
    log::info!(target: "HID", "Device disconnected, resetting shared state");
    *shared.device.write().unwrap_or_reset("device") = crate::engine::state::DeviceState::default();
    *shared
        .pipeline
        .tablet_data
        .write()
        .unwrap_or_reset("tablet_data") = crate::drivers::TabletData::default();
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::drivers::NextTabletDriver;
    use crate::engine::state::EngineStatus;
    use crossbeam_channel::bounded;

    struct StubDriver;

    impl NextTabletDriver for StubDriver {
        fn get_name(&self) -> &'static str {
            "Stub Tablet"
        }
        fn get_specs(&self) -> (f32, f32, f32) {
            (16000.0, 10000.0, 8191.0)
        }
        fn get_physical_specs(&self) -> (f32, f32) {
            (160.0, 100.0)
        }
        fn get_vid_pid(&self) -> (u16, u16) {
            (0x1234, 0x5678)
        }
        fn parse(&self, _data: &[u8]) -> Option<TabletData> {
            None
        }
    }

    fn shared(first_run: bool) -> Arc<SharedState> {
        let shared = Arc::new(SharedState::new());
        *shared.lifecycle.is_first_run.write().unwrap() = first_run;
        shared
    }

    #[test]
    fn connecting_a_tablet_publishes_its_metadata() {
        let shared = shared(false);
        let mut local = MappingConfig::default();
        on_device_connected(&shared, &StubDriver, 0x1234, 0x5678, &mut local);

        let device = shared.device.read().unwrap().clone();
        assert_eq!(device.name, "Stub Tablet");
        assert_eq!((device.vid, device.pid), (0x1234, 0x5678));
        assert_eq!(device.physical_size, (160.0, 100.0));
        assert_eq!(device.hardware_size, (16000.0, 10000.0));
        assert_eq!(device.max_pressure, 8191.0);
        // Not the first run: the user's active area is left alone.
        assert_eq!(shared.config.version.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_first_connection_centres_the_active_area_on_the_tablet() {
        let shared = shared(true);
        let mut local = MappingConfig::default();
        on_device_connected(&shared, &StubDriver, 0x1234, 0x5678, &mut local);

        let area = shared.config.mapping.read().unwrap().active_area;
        assert_eq!((area.w, area.h, area.x, area.y), (160.0, 100.0, 80.0, 50.0));
        assert_eq!(local.active_area.w, 160.0);
        assert!(!*shared.lifecycle.is_first_run.read().unwrap());
        assert_eq!(shared.config.version.load(Ordering::SeqCst), 1);

        // A second connection no longer touches the configuration.
        let mut again = MappingConfig::default();
        on_device_connected(&shared, &StubDriver, 0x1234, 0x5678, &mut again);
        assert_eq!(shared.config.version.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn disconnecting_restores_the_empty_device_and_clears_the_last_packet() {
        let shared = shared(false);
        let mut local = MappingConfig::default();
        on_device_connected(&shared, &StubDriver, 0x1234, 0x5678, &mut local);
        shared.pipeline.tablet_data.write().unwrap().is_connected = true;

        on_disconnected(&shared);

        assert_eq!(shared.device.read().unwrap().vid, 0);
        assert_eq!(shared.device.read().unwrap().name, "No Tablet Detected");
        assert!(!shared.pipeline.tablet_data.read().unwrap().is_connected);
    }

    #[test]
    fn the_filter_pipeline_starts_empty_without_plugins() {
        let shared = shared(false);
        let pipeline = init_filter_pipeline(&shared, &MappingConfig::default());
        assert!(pipeline.entries.is_empty());
    }

    #[test]
    fn raising_the_thread_priority_never_fails_the_caller() {
        init_thread_priority();
    }

    // The two tests below run the whole owner setup with the loop already told to stop: HID API,
    // injector, filters, shared segment and command socket come up and go down, and no tablet is
    // ever detected or opened.

    #[test]
    fn an_owner_started_during_shutdown_sets_up_and_tears_down_cleanly() {
        let shared = shared(false);
        shared
            .lifecycle
            .shutdown_requested
            .store(true, Ordering::Relaxed);
        let (sender, _receiver) = bounded(1);
        owner_iteration(&shared, &sender);
        let status = shared.lifecycle.engine_status.read().unwrap().clone();
        assert!(
            matches!(status, EngineStatus::Running | EngineStatus::Failed(_)),
            "{status:?}"
        );
        assert_eq!(shared.device.read().unwrap().vid, 0);
    }

    #[test]
    fn a_reload_request_restarts_the_owner_context() {
        let shared = shared(false);
        shared
            .config
            .reload_requested
            .store(true, Ordering::Relaxed);
        let (sender, _receiver) = bounded(1);
        owner_iteration(&shared, &sender);
        // The request is consumed by the loop that honours it (unless setup failed first).
        let status = shared.lifecycle.engine_status.read().unwrap().clone();
        if matches!(status, EngineStatus::Running) {
            assert!(!shared.config.reload_requested.load(Ordering::Relaxed));
        }
    }
}
