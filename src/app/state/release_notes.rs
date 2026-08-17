//! State and background fetch kickoff for the dynamically-fetched Release Notes tab.
//!
//! The actual network fetching/caching lives in [`crate::app::release_notes`];
//! this module only tracks the fetch's UI-facing status and result handoff.

use super::TabletMapperApp;

/// Outcome of the last release notes fetch, driving what the Release tab renders.
pub enum ReleaseNotesStatus {
    /// No fetch has been requested yet.
    Idle,
    /// A background fetch is currently in flight.
    Loading,
    /// Releases are available, either fresh from the network or from the local cache.
    Loaded {
        releases: Vec<crate::app::autoupdate::Release>,
        from_cache: bool,
    },
    /// The fetch failed and no local cache exists.
    Unavailable,
}

/// State for the dynamically-fetched Release Notes tab.
pub struct ReleaseNotesState {
    /// Current fetch/render status.
    pub status: ReleaseNotesStatus,
    /// Result of the last background fetch, picked up on the next frame.
    pub pending:
        std::sync::Arc<std::sync::Mutex<Option<crate::app::release_notes::ReleaseNotesOutcome>>>,
}

impl TabletMapperApp {
    /// Kicks off a background fetch of the release notes, unless one is already
    /// in flight. Called once when the user switches into the Release tab.
    pub fn request_release_notes_fetch(&mut self, ctx: &eframe::egui::Context) {
        if matches!(self.release_notes.status, ReleaseNotesStatus::Loading) {
            return;
        }
        self.release_notes.status = ReleaseNotesStatus::Loading;
        let pending = std::sync::Arc::clone(&self.release_notes.pending);
        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            let outcome = crate::app::release_notes::get_releases();
            if let Ok(mut guard) = pending.lock() {
                *guard = Some(outcome);
            }
            ctx_clone.request_repaint();
        });
    }
}
