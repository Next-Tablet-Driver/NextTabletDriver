use std::sync::Arc;
use crate::engine::state::SharedState;
use crate::core::config::models::MappingConfig;
use crossbeam_channel::{Sender, Receiver};
use eframe::egui::Context;

pub struct ThreadSupervisor;

impl ThreadSupervisor {
    pub fn spawn_engine(shared: Arc<SharedState>, ctx: Context, sender: Sender<crate::drivers::TabletData>) {
        log::info!(target: "App", "Spawning Input Engine thread");
        std::thread::spawn(move || {
            crate::engine::tablet_manager::run_manager(shared, ctx, sender);
        });
    }

    pub fn spawn_websocket(shared: Arc<SharedState>) {
        log::info!(target: "WebSocket", "Spawning WebSocket thread");
        std::thread::spawn(move || {
            crate::app::websocket::websocket_loop(shared);
        });
    }

    pub fn spawn_saver(receiver: Receiver<MappingConfig>) {
        log::info!(target: "Config", "Spawning Background Saver thread");
        std::thread::spawn(move || {
            while let Ok(cfg) = receiver.recv() {
                let mut latest = cfg;
                while let Ok(newer) = receiver.try_recv() {
                    latest = newer;
                }
                if let Err(e) = crate::settings::save_last_session(&latest) {
                    log::error!(target: "Config", "Background saver failed: {}", e);
                }
            }
        });
    }
}
