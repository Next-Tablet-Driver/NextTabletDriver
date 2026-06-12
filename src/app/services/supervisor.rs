use crate::core::config::models::MappingConfig;
use crate::engine::state::SharedState;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;

pub struct ThreadSupervisor;

impl ThreadSupervisor {
    pub fn spawn_engine(shared: Arc<SharedState>, sender: Sender<crate::drivers::TabletData>) {
        log::info!(target: "App", "Spawning Input Engine thread");
        std::thread::spawn(move || {
            crate::engine::tablet_manager::run_manager(&shared, &sender);
        });
    }

    pub fn spawn_websocket(shared: Arc<SharedState>) {
        log::info!(target: "WebSocket", "Spawning WebSocket thread");
        std::thread::spawn(move || {
            crate::app::websocket::websocket_loop(&shared);
        });
    }

    pub fn spawn_saver(receiver: Receiver<MappingConfig>) {
        log::info!(target: "Config", "Spawning Background Saver thread");
        std::thread::spawn(move || {
            while let Ok(cfg) = receiver.recv() {
                // Debounce: Wait 500ms to accumulate rapid consecutive events
                std::thread::sleep(std::time::Duration::from_millis(500));

                let mut latest = cfg;
                while let Ok(newer) = receiver.try_recv() {
                    latest = newer;
                }
                if let Err(e) = crate::settings::save_last_session(&latest) {
                    log::error!(target: "Config", "Background saver failed: {e}");
                }
            }
        });
    }
}
