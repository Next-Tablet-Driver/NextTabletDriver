use crate::core::config::models::MappingConfig;
use crate::engine::state::SharedState;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

/// How long the background saver waits for further changes before writing the session.
const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

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
            crate::application::services::websocket::websocket_loop(&shared);
        });
    }

    pub fn spawn_saver(receiver: Receiver<MappingConfig>) {
        Self::spawn_saver_with(receiver, SAVE_DEBOUNCE, crate::settings::save_last_session);
    }

    /// The saver thread: waits `debounce` after a change, keeps only the latest configuration
    /// received meanwhile and hands it to `save`. Stops when every sender is gone.
    fn spawn_saver_with<F>(receiver: Receiver<MappingConfig>, debounce: Duration, save: F)
    where
        F: Fn(&MappingConfig) -> Result<(), String> + Send + 'static,
    {
        log::info!(target: "Config", "Spawning Background Saver thread");
        std::thread::spawn(move || {
            while let Ok(cfg) = receiver.recv() {
                // Debounce: wait to accumulate rapid consecutive events
                std::thread::sleep(debounce);

                let mut latest = cfg;
                while let Ok(newer) = receiver.try_recv() {
                    latest = newer;
                }
                if let Err(e) = save(&latest) {
                    log::error!(target: "Config", "Background saver failed: {e}");
                }
            }
        });
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crossbeam_channel::{bounded, unbounded};

    fn config(width: f32) -> MappingConfig {
        let mut config = MappingConfig::default();
        config.active_area.w = width;
        config
    }

    #[test]
    fn rapid_changes_are_coalesced_into_one_save_of_the_latest() {
        let (sender, receiver) = unbounded();
        let (saved_tx, saved_rx) = bounded(8);
        ThreadSupervisor::spawn_saver_with(receiver, Duration::from_millis(150), move |cfg| {
            saved_tx.send(cfg.active_area.w).unwrap();
            Ok(())
        });

        for width in [10.0, 20.0, 30.0] {
            sender.send(config(width)).unwrap();
        }
        assert_eq!(saved_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 30.0);
        // Nothing else follows: the earlier values were dropped, not saved late.
        assert!(saved_rx.recv_timeout(Duration::from_millis(400)).is_err());
    }

    #[test]
    fn a_later_change_is_saved_on_its_own() {
        let (sender, receiver) = unbounded();
        let (saved_tx, saved_rx) = bounded(8);
        ThreadSupervisor::spawn_saver_with(receiver, Duration::from_millis(20), move |cfg| {
            saved_tx.send(cfg.active_area.w).unwrap();
            Ok(())
        });

        sender.send(config(11.0)).unwrap();
        assert_eq!(saved_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 11.0);
        sender.send(config(22.0)).unwrap();
        assert_eq!(saved_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 22.0);
    }

    #[test]
    fn a_failing_save_does_not_stop_the_saver() {
        crate::test_support::evaluate_log_arguments();
        let (sender, receiver) = unbounded();
        let (attempts_tx, attempts_rx) = bounded(8);
        ThreadSupervisor::spawn_saver_with(receiver, Duration::from_millis(20), move |cfg| {
            attempts_tx.send(cfg.active_area.w).unwrap();
            Err("disk full".to_string())
        });

        sender.send(config(1.0)).unwrap();
        assert_eq!(
            attempts_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            1.0
        );
        sender.send(config(2.0)).unwrap();
        assert_eq!(
            attempts_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            2.0
        );
    }

    #[test]
    fn the_saver_ends_when_every_sender_is_gone() {
        let (sender, receiver) = unbounded::<MappingConfig>();
        let (saved_tx, saved_rx) = bounded::<f32>(1);
        ThreadSupervisor::spawn_saver_with(receiver, Duration::from_millis(10), move |cfg| {
            let _ = saved_tx.send(cfg.active_area.w);
            Ok(())
        });
        drop(sender);
        // The closure (and with it `saved_tx`) is dropped when the thread ends, which is what
        // disconnects the channel; a plain timeout would mean the saver is still running.
        assert_eq!(
            saved_rx.recv_timeout(Duration::from_secs(5)),
            Err(crossbeam_channel::RecvTimeoutError::Disconnected)
        );
    }
}
