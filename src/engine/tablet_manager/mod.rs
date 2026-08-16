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

use crate::drivers::TabletData;
use crate::engine::interop::lock::try_acquire_hid_owner;
use crate::engine::state::SharedState;
use crossbeam_channel::Sender;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

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
        owner::owner_iteration(shared_clone, sender_clone);
    } else {
        reader::reader_iteration(shared_clone, sender_clone);
    }
}
