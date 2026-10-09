//! Runs this process as a non-owner: mirrors another process's published
//! HID-owner state instead of touching a real device.

use super::owner::owner_iteration;
use super::sdk_bridge::apply_shm_snapshot;
use super::try_acquire_boxed;
use crate::drivers::TabletData;
use crate::engine::interop::shm::ShmReader;
use crate::engine::state::SharedState;
use crossbeam_channel::Sender;
use std::any::Any;
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
    reader_loop(
        shared,
        sender,
        OWNER_PROMOTION_RETRY_INTERVAL,
        &try_acquire_boxed,
        &owner_iteration,
    );
}

/// The reader loop, with its two OS-bound collaborators passed in: how to try to become the HID
/// owner (`try_acquire`, whose guard is held until the loop ends) and what to run once promoted
/// (`take_over`).
fn reader_loop(
    shared: &Arc<SharedState>,
    sender: &Sender<TabletData>,
    promotion_retry_interval: Duration,
    try_acquire: &dyn Fn() -> Option<Box<dyn Any>>,
    take_over: &dyn Fn(&Arc<SharedState>, &Sender<TabletData>),
) {
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

        if Instant::now().duration_since(last_promotion_attempt) >= promotion_retry_interval {
            last_promotion_attempt = Instant::now();
            // Held for the rest of this function's life, same as the
            // top-level branch in `manager_thread_iteration`.
            if let Some(_hid_owner) = try_acquire() {
                log::info!(target: "TabletManager", "Promoted to HID owner, taking over the real device");
                take_over(shared, sender);
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::engine::interop::shm::{SdkPublicState, ShmWriter};
    use crossbeam_channel::bounded;

    fn shared() -> Arc<SharedState> {
        log::set_max_level(log::LevelFilter::Trace);
        Arc::new(SharedState::new())
    }

    #[test]
    fn a_reader_stops_at_once_when_a_shutdown_was_requested() {
        let shared = shared();
        shared
            .lifecycle
            .shutdown_requested
            .store(true, Ordering::Relaxed);
        let (sender, _receiver) = bounded(1);
        reader_iteration(&shared, &sender);
    }

    #[test]
    fn a_reload_request_ends_the_iteration_and_is_consumed() {
        let shared = shared();
        shared
            .config
            .reload_requested
            .store(true, Ordering::Relaxed);
        let (sender, _receiver) = bounded(1);
        reader_iteration(&shared, &sender);
        assert!(!shared.config.reload_requested.load(Ordering::Relaxed));
    }

    /// What a reader runs once promoted: it only records that it did.
    fn mark(flag: &std::cell::Cell<bool>) -> impl Fn(&Arc<SharedState>, &Sender<TabletData>) {
        move |_, _| flag.set(true)
    }

    /// Asks the reader to stop once `condition` holds (or after a generous timeout, so a failing
    /// test ends instead of hanging).
    fn stop_when(
        shared: &Arc<SharedState>,
        condition: Box<dyn Fn() -> bool + Send>,
    ) -> thread::JoinHandle<()> {
        let shared = Arc::clone(shared);
        thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !condition() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            shared
                .lifecycle
                .shutdown_requested
                .store(true, Ordering::Relaxed);
        })
    }

    #[test]
    fn a_reader_takes_over_as_soon_as_it_can_become_the_owner() {
        let shared = shared();
        let (sender, _receiver) = bounded(1);
        let promoted = std::cell::Cell::new(false);
        reader_loop(
            &shared,
            &sender,
            Duration::ZERO,
            &|| Some(Box::new(()) as Box<dyn Any>),
            &mark(&promoted),
        );
        assert!(promoted.get());
    }

    #[test]
    fn the_reader_mirrors_the_published_state_until_it_is_stopped() {
        let writer = ShmWriter::create().expect("the shared segment can be created");
        writer.publish(&SdkPublicState {
            is_connected: true,
            status: 3,
            vid: 0x056A,
            pid: 0x037A,
            config_version: 4,
            ..SdkPublicState::default()
        });

        let shared = shared();
        let probe = Arc::clone(&shared);
        let stopper = stop_when(
            &shared,
            Box::new(move || probe.device.read().unwrap().vid == 0x056A),
        );
        let (sender, _receiver) = bounded(1);
        reader_iteration(&shared, &sender);
        stopper.join().unwrap();

        assert_eq!(shared.device.read().unwrap().vid, 0x056A);
        assert!(shared.pipeline.tablet_data.read().unwrap().is_connected);
        assert_eq!(shared.config.version.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn a_reader_keeps_mirroring_while_the_owner_lock_stays_taken() {
        let shared = shared();
        let (sender, _receiver) = bounded(1);
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let promoted = std::cell::Cell::new(false);
        let counted = Arc::clone(&attempts);
        let stopper = stop_when(
            &shared,
            Box::new(move || counted.load(Ordering::Relaxed) >= 3),
        );
        let counting = Arc::clone(&attempts);
        reader_loop(
            &shared,
            &sender,
            Duration::ZERO,
            &move || {
                counting.fetch_add(1, Ordering::Relaxed);
                None
            },
            &mark(&promoted),
        );
        stopper.join().unwrap();
        assert!(attempts.load(Ordering::Relaxed) >= 3);
        assert!(!promoted.get());
    }

    #[test]
    fn a_reader_started_before_any_owner_picks_the_segment_up_later() {
        let shared = shared();
        let (sender, _receiver) = bounded(1);
        let promoted = std::cell::Cell::new(false);
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let owner_done = Arc::clone(&done);
        let owner = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            let writer = ShmWriter::create().expect("the shared segment can be created");
            writer.publish(&SdkPublicState {
                is_connected: true,
                vid: 0x1111,
                pid: 0x2222,
                ..SdkPublicState::default()
            });
            // Keep the segment alive until the reader is finished with it.
            let deadline = Instant::now() + Duration::from_secs(10);
            while !owner_done.load(Ordering::Relaxed) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
        });
        let probe = Arc::clone(&shared);
        let stopper = stop_when(
            &shared,
            Box::new(move || probe.device.read().unwrap().vid == 0x1111),
        );
        reader_loop(
            &shared,
            &sender,
            Duration::from_secs(3600),
            &|| None,
            &mark(&promoted),
        );
        stopper.join().unwrap();
        done.store(true, Ordering::Relaxed);
        owner.join().unwrap();
        assert_eq!(shared.device.read().unwrap().vid, 0x1111);
        assert!(!promoted.get());
    }
}
