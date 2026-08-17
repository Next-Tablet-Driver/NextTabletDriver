//! Auto-updater state transitions: kicking off a check, starting a download,
//! or dismissing the result. The actual network/install work happens in
//! [`crate::app::autoupdate`] on a background thread; this module only
//! manages `TabletMapperApp::update_status` around it.

use super::TabletMapperApp;
use crate::app::autoupdate::UpdateStatus;

impl TabletMapperApp {
    pub fn start_update(&mut self) {
        if let UpdateStatus::Available(release) = &self.update_status {
            let release_clone = release.clone();
            let sender = self.update_sender.clone();
            std::thread::spawn(move || {
                if let Err(e) =
                    crate::app::autoupdate::download_and_install(&release_clone, &sender)
                {
                    let _ = sender.send(UpdateStatus::Error(e.to_string()));
                }
            });
            self.update_status = UpdateStatus::Downloading(
                crate::app::autoupdate::models::DownloadProgress::default(),
            );
        }
    }

    pub fn dismiss_update(&mut self) {
        self.update_status = UpdateStatus::Idle;
    }

    pub fn check_for_updates(&mut self) {
        let sender = self.update_sender.clone();
        self.update_status = UpdateStatus::Checking;
        std::thread::spawn(move || match crate::app::autoupdate::check_for_updates() {
            Ok(Some(release)) => {
                let _ = sender.send(UpdateStatus::Available(release));
            }
            Ok(None) => {
                let _ = sender.send(UpdateStatus::Idle);
            }
            Err(e) => {
                let _ = sender.send(UpdateStatus::Error(e.to_string()));
            }
        });
    }
}
