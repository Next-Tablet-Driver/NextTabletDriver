pub mod models;
pub mod snapshot;

mod console;
mod profile;
mod release_notes;
mod theme_store;
mod updater;

pub use console::ConsoleState;
pub use models::*;
pub use release_notes::{ReleaseNotesState, ReleaseNotesStatus};
pub use snapshot::*;
pub use theme_store::{ThemeStoreItem, ThemeStoreResult, ThemeStoreState};

use crate::core::config::models::MappingConfig;
use crossbeam_channel::Receiver;
use display_info::DisplayInfo;
use std::sync::Arc;
use std::time::Instant;

use crate::app::autoupdate::UpdateStatus;
use crate::drivers::TabletData;
use crate::engine::state::SharedState;

/// The core application state structure used by the `eframe` (egui) integration.
#[allow(clippy::struct_excessive_bools)]
pub struct TabletMapperApp {
    /// Reference-counted handle to the thread-safe engine/driver state.
    pub shared: Arc<SharedState>,

    /// List of detected physical monitors/displays on the host system.
    pub displays: Vec<DisplayInfo>,
    /// Instant of the last update/repaint cycle.
    pub last_update: Instant,
    /// Instant when the active configuration was last logged.
    pub last_config_log: Instant,
    /// Tracks profile state identity (name, filepath, and saved/unsaved status).
    pub profile: ProfileState,
    /// The currently selected app tab/view in the UI panel.
    pub active_tab: AppTab,
    /// The active tab as of the previous frame, used to detect tab-switch edges.
    pub previous_tab: AppTab,

    /// Channel receiver for tablet input packets streamed from the driver thread.
    pub tablet_receiver: Receiver<TabletData>,
    /// Channel receiver for software auto-update statuses.
    pub update_receiver: Receiver<UpdateStatus>,
    /// Channel sender for publishing auto-update status events.
    pub update_sender: crossbeam_channel::Sender<UpdateStatus>,
    /// Current cached auto-update status.
    pub update_status: UpdateStatus,

    /// Channel sender to trigger asynchronous config writes to the settings file.
    pub save_sender: crossbeam_channel::Sender<MappingConfig>,

    /// Queue of active toast notifications rendered in the overlay.
    pub toasts: Vec<Toast>,

    /// Display name of the filter currently selected in the Filters tab.
    pub selected_filter: String,

    /// Toggle to render the floating developer debug details panel.
    pub show_debugger: bool,
    /// Toggle to display real-time latency statistics.
    pub show_latency_stats: bool,
    /// Real-time frame paint speed and packet latency diagnostics.
    pub metrics: Metrics,

    /// Remembers if the window was minimized in the last update frame.
    pub was_minimized: bool,

    /// State for the console/log viewer tab.
    pub console: ConsoleState,

    /// State for the online theme store and background theme downloads.
    pub theme_store: ThemeStoreState,

    /// State for the dynamically-fetched Release Notes tab.
    pub release_notes: ReleaseNotesState,

    /// Toggle to render the close confirmation dialog modal.
    pub show_close_confirm: bool,
    /// If true, bypasses close confirmation dialog and exits immediately.
    pub force_close: bool,

    /// Set to true on Linux if the required udev rules are not installed.
    pub missing_udev_rules: bool,

    /// Application-level preferences (theme, language) stored separately from tablet config.
    pub app_prefs: crate::settings::app_preferences::AppPreferences,
}

const MAX_TOASTS: usize = 3;

impl TabletMapperApp {
    /// Pushes a new toast notification message into the display queue.
    ///
    /// Deduplicates duplicate messages and respects `MAX_TOASTS`.
    pub fn push_toast(&mut self, message: String, level: ToastLevel) {
        if self.toasts.iter().any(|t| t.message == message) {
            return;
        }
        if self.toasts.len() >= MAX_TOASTS {
            self.toasts.remove(0);
        }
        self.toasts.push(Toast {
            message,
            level,
            created_at: Instant::now(),
        });
    }
}
