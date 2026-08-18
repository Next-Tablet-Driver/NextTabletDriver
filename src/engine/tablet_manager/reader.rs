//! Runs this process as a non-owner: mirrors another process's published
//! HID-owner state instead of touching a real device.

use super::owner::owner_iteration;
use super::sdk_bridge::apply_shm_snapshot;
use crate::drivers::TabletData;
use crate::engine::interop::lock::try_acquire_hid_owner;
use crate::engine::interop::shm::ShmReader;
use crate::engine::state::SharedState;
use crossbeam_channel::Sender;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

/// How often a reader retries becoming the HID owner (e.g. the previous
/// owner exited) and how often it polls the shared segment for fresh state.
const OWNER_PROMOTION_RETRY_INTERVAL: Duration = Duration::from_secs(3);
const SHM_READER_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Runs this process as a non-owner: mirrors the current HID owner's
/// published state into the local `SharedState` instead of touching a real
/// device, and periodically retries promotion to owner.
pub(super) fn reader_iteration(shared: &Arc<SharedState>, sender: &Sender<TabletData>) {
    log::info!(target: "TabletManager", "Another process owns the HID device; running in reader mode");

    let mut reader = ShmReader::open();
    let mut last_config_version = None;
    let mut last_promotion_attempt = Instant::now();

    loop {
        if shared.lifecycle.shutdown_requested.load(Ordering::Relaxed) {
            return;
        }
        if shared
            .config
            .reload_requested
            .swap(false, Ordering::Relaxed)
        {
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
