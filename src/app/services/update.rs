use crate::app::autoupdate::{self, UpdateStatus};
use crossbeam_channel::{Receiver, Sender, bounded};

pub struct UpdateService {
    pub receiver: Receiver<UpdateStatus>,
    pub sender: Sender<UpdateStatus>,
}

impl Default for UpdateService {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateService {
    #[must_use]
    pub fn new() -> Self {
        let (update_sender, update_receiver) = bounded(1);
        Self {
            receiver: update_receiver,
            sender: update_sender,
        }
    }

    pub fn start_check(&self) {
        let sender = self.sender.clone();
        log::info!(target: "App", "Spawning Auto-Updater thread");
        std::thread::spawn(move || match autoupdate::check_for_updates() {
            Ok(Some(release)) => {
                let _ = sender.send(UpdateStatus::Available(release));
            }
            Ok(None) => {}
            Err(e) => {
                log::error!(target: "Update", "Failed to check for updates: {e}");
            }
        });
    }
}
