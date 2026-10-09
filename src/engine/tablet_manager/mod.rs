//! # Tablet Device Manager
//!
//! This module is the execution environment for the background USB polling thread.
//! It handles detecting devices, reading raw USB packets, checking for configuration
//! updates, and feeding data to the UI thread and [`Pipeline`](crate::engine::pipeline::Pipeline).
//!
//! # Architecture
//!
//! ```text
//! run_manager()
//!   └── manager_thread_iteration()
//!         ├── owner::owner_iteration()      (this process holds the HID owner lock)
//!         │    ├── init_thread_priority()
//!         │    ├── init_filter_pipeline()
//!         │    └── loop
//!         │          ├── on_device_connected()
//!         │          ├── polling::run_polling_loop()
//!         │          │    ├── process_packet()      (publishes shm state)
//!         │          │    └── maybe_reload_config()
//!         │          └── on_disconnected()
//!         └── reader::reader_iteration()     (another process owns the HID device)
//!              └── loop
//!                    ├── sdk_bridge::apply_shm_snapshot()
//!                    └── try_acquire_hid_owner()      (periodic promotion retry)
//! ```
//!
//! The responsibilities above are split across submodules: [`owner`] runs this
//! process as the HID owner, [`polling`] drives the raw packet read/process
//! loop, [`reader`] mirrors another process's published state, and
//! [`sdk_bridge`] publishes/consumes state through `engine::interop`'s
//! SHM/command layer. See `engine::interop` for the HID-owner arbitration
//! mechanism: exactly one process (this desktop app, or an SDK-embedded game)
//! opens the real HID device at a time, and every other process mirrors its
//! state instead.

mod owner;
mod polling;
mod reader;
mod sdk_bridge;

/// Hooks for `qa/benches`; not a stable API.
#[doc(hidden)]
pub mod bench_support {
    pub use super::polling::{PacketLoopState, process_packet_for_bench};
}

use crate::drivers::TabletData;
use crate::engine::interop::lock::try_acquire_hid_owner;
use crate::engine::state::SharedState;
use crossbeam_channel::Sender;
use std::any::Any;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

/// Starts the background USB polling loop.
pub fn run_manager(shared: &Arc<SharedState>, tablet_sender: &Sender<TabletData>) {
    run_manager_loop(
        shared,
        tablet_sender,
        Duration::from_secs(1),
        &manager_thread_iteration,
    );
}

/// Runs `iteration` again after every crash or end of context, until a shutdown is requested.
fn run_manager_loop(
    shared: &Arc<SharedState>,
    tablet_sender: &Sender<TabletData>,
    restart_delay: Duration,
    iteration: &dyn Fn(&Arc<SharedState>, &Sender<TabletData>),
) {
    log::info!(target: "TabletManager", "Starting device manager thread");

    loop {
        let shared_clone = Arc::clone(shared);
        let sender_clone = tablet_sender.clone();

        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            iteration(&shared_clone, &sender_clone);
        }));

        if let Err(err) = result {
            log::error!(target: "TabletManager", "THREAD CRASHED: {err:?}");
        }

        if shared.lifecycle.shutdown_requested.load(Ordering::Relaxed) {
            break;
        }

        log::warn!(target: "TabletManager", "Engine context terminated, restarting in {restart_delay:?}...");
        thread::sleep(restart_delay);
    }
}

fn manager_thread_iteration(shared_clone: &Arc<SharedState>, sender_clone: &Sender<TabletData>) {
    manager_iteration_with(
        shared_clone,
        sender_clone,
        &try_acquire_boxed,
        &owner::owner_iteration,
        &reader::reader_iteration,
    );
}

/// Tries to become the HID owner; the guard is boxed so callers do not depend on its type.
fn try_acquire_boxed() -> Option<Box<dyn Any>> {
    try_acquire_hid_owner().map(|guard| Box::new(guard) as Box<dyn Any>)
}

/// Runs `owner` if `try_acquire` grants the HID owner lock, `reader` otherwise.
fn manager_iteration_with(
    shared: &Arc<SharedState>,
    sender: &Sender<TabletData>,
    try_acquire: &dyn Fn() -> Option<Box<dyn Any>>,
    owner: &dyn Fn(&Arc<SharedState>, &Sender<TabletData>),
    reader: &dyn Fn(&Arc<SharedState>, &Sender<TabletData>),
) {
    // The binding below is held for the entire branch body, which is exactly
    // as long as this process should keep the real HID device open.
    if let Some(_hid_owner) = try_acquire() {
        owner(shared, sender);
    } else {
        reader(shared, sender);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;

    #[test]
    fn only_one_guard_can_own_the_device_at_a_time() {
        let first = try_acquire_boxed();
        assert!(first.is_some());
        // The lock belongs to the thread that took it, so a rival has to be another thread.
        let rival = thread::spawn(|| try_acquire_boxed().is_some())
            .join()
            .unwrap();
        assert!(!rival);
        drop(first);
        let successor = thread::spawn(|| try_acquire_boxed().is_some())
            .join()
            .unwrap();
        assert!(successor);
    }

    // These run the real manager with a shutdown already requested: the iteration sets up and
    // tears down (HID API, injector, shared segment, command socket) but never opens a tablet.

    #[test]
    fn the_manager_returns_once_a_shutdown_was_requested() {
        let shared = Arc::new(SharedState::new());
        shared
            .lifecycle
            .shutdown_requested
            .store(true, Ordering::Relaxed);
        let (sender, _receiver) = bounded(1);
        run_manager(&shared, &sender);
        assert!(shared.lifecycle.shutdown_requested.load(Ordering::Relaxed));
    }

    #[test]
    fn a_single_iteration_leaves_no_tablet_in_the_shared_state() {
        let shared = Arc::new(SharedState::new());
        shared
            .lifecycle
            .shutdown_requested
            .store(true, Ordering::Relaxed);
        let (sender, _receiver) = bounded(1);
        manager_thread_iteration(&shared, &sender);
        assert_eq!(shared.device.read().unwrap().vid, 0);
        assert!(!shared.pipeline.tablet_data.read().unwrap().is_connected);
    }

    fn quiet_state() -> (Arc<SharedState>, Sender<TabletData>) {
        crate::test_support::evaluate_log_arguments();
        let (sender, _receiver) = bounded(1);
        (Arc::new(SharedState::new()), sender)
    }

    type Ran = std::cell::RefCell<Vec<&'static str>>;

    /// A branch of the manager that only records that it ran.
    fn recorder(ran: &Ran, name: &'static str) -> impl Fn(&Arc<SharedState>, &Sender<TabletData>) {
        move |_, _| ran.borrow_mut().push(name)
    }

    #[test]
    fn the_owner_runs_when_the_lock_is_granted() {
        let (shared, sender) = quiet_state();
        let ran = Ran::default();
        manager_iteration_with(
            &shared,
            &sender,
            &|| Some(Box::new(()) as Box<dyn Any>),
            &recorder(&ran, "owner"),
            &recorder(&ran, "reader"),
        );
        assert_eq!(*ran.borrow(), ["owner"]);
    }

    #[test]
    fn the_reader_runs_when_another_process_owns_the_device() {
        let (shared, sender) = quiet_state();
        let ran = Ran::default();
        manager_iteration_with(
            &shared,
            &sender,
            &|| None,
            &recorder(&ran, "owner"),
            &recorder(&ran, "reader"),
        );
        assert_eq!(*ran.borrow(), ["reader"]);
    }

    #[test]
    fn a_crashed_iteration_is_restarted_until_a_shutdown_is_requested() {
        let (shared, sender) = quiet_state();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        run_manager_loop(&shared, &sender, Duration::ZERO, &|shared, _| {
            assert!(
                calls.fetch_add(1, Ordering::Relaxed) != 0,
                "the first iteration crashes"
            );
            shared
                .lifecycle
                .shutdown_requested
                .store(true, Ordering::Relaxed);
        });
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn an_iteration_that_ends_normally_is_restarted_too() {
        let (shared, sender) = quiet_state();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        run_manager_loop(&shared, &sender, Duration::ZERO, &|shared, _| {
            if calls.fetch_add(1, Ordering::Relaxed) == 2 {
                shared
                    .lifecycle
                    .shutdown_requested
                    .store(true, Ordering::Relaxed);
            }
        });
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }
}
