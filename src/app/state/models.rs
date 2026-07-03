use crate::core::config::models::MappingConfig;
use crate::t;
use std::path::PathBuf;
use std::time::Instant;

/// Represents the currently active tab in the main application window.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppTab {
    /// Target coordinates mapping tab.
    Output,
    /// Configuration for antichatter and stats filters.
    Filters,
    /// Settings for pen bindings and thresholds.
    PenSettings,
    /// Built-in console log viewer.
    Console,
    /// General application and tray settings.
    Settings,
    /// Release notes and update checker.
    Release,
}

/// Tracks the state of the currently active user profile.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileState {
    /// Human-readable profile display name.
    pub name: String,
    /// Absolute path to the profile configuration file on disk.
    pub path: Option<PathBuf>,
    /// The last saved configuration state. Used to detect unsaved changes.
    pub last_saved: MappingConfig,
}

impl ProfileState {
    /// Returns true if the current UI configuration differs from the last saved state.
    #[must_use]
    pub fn is_dirty(&self, current: &MappingConfig) -> bool {
        *current != self.last_saved
    }

    /// Generates a display name for the profile, prefixing with an asterisk if modified.
    #[must_use]
    pub fn display_name(&self, current: &MappingConfig) -> String {
        let base = if self.path.is_some() {
            self.name.clone()
        } else {
            t!("footer.unsaved_session")
        };

        if self.is_dirty(current) {
            format!("*{base}")
        } else {
            base
        }
    }

    /// Marks the given configuration as saved on disk, syncing it with `last_saved`.
    pub fn mark_saved(&mut self, config: &MappingConfig) {
        self.last_saved = config.clone();
    }
}

/// Severity level for UI toast notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    /// Informational message.
    Info,
    /// Non-fatal warning alert.
    Warning,
    /// Fatal or operation failure error.
    Error,
}

/// A transient notification displayed in the UI overlay.
#[derive(Clone, Debug)]
pub struct Toast {
    /// The message text to display.
    pub message: String,
    /// The severity level determining color and icon.
    pub level: ToastLevel,
    /// Monotonic timestamp when this notification was created.
    pub created_at: Instant,
}

/// Encapsulates performance metrics and latency tracking.
#[derive(Clone, Debug)]
pub struct Metrics {
    /// The calculated packet rate (in Hz) displayed in the UI.
    pub displayed_hz: f32,
    /// Monotonic timestamp of the last Hz calculation update.
    pub last_hz_update: Instant,
    /// The packet count snapshot recorded during the last Hz update.
    pub last_packet_count: u32,
    /// The duration (in ms) of the most recent UI frame layout/render.
    pub ui_latency_ms: f32,
    /// The minimum recorded UI frame render latency (in ms).
    pub min_ui_latency_ms: f32,
    /// The maximum recorded UI frame render latency (in ms).
    pub max_ui_latency_ms: f32,
    /// Exponential moving average of the UI frame render latency (in ms).
    pub avg_ui_latency_ms: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            displayed_hz: 0.0,
            last_hz_update: Instant::now(),
            last_packet_count: 0,
            ui_latency_ms: 0.0,
            min_ui_latency_ms: f32::MAX,
            max_ui_latency_ms: 0.0,
            avg_ui_latency_ms: 0.0,
        }
    }
}

impl Metrics {
    /// Updates the calculated packet rate (Hz) using the current raw USB packet counter.
    ///
    /// The frequency is smoothed over time using a simple low-pass filter.
    pub fn update_hz(&mut self, current_packets: u32) {
        let elapsed = self.last_hz_update.elapsed();
        if elapsed >= std::time::Duration::from_millis(200) {
            let delta = current_packets.saturating_sub(self.last_packet_count);
            let hz = delta as f32 / elapsed.as_secs_f32();
            self.displayed_hz = hz.mul_add(0.3, self.displayed_hz);
            self.last_packet_count = current_packets;
            self.last_hz_update = Instant::now();
        }
    }

    /// Records a new UI frame latency sample, updating minimum, maximum, and average values.
    pub fn update_latency(&mut self, latency: f32) {
        self.ui_latency_ms = latency;
        self.min_ui_latency_ms = self.min_ui_latency_ms.min(latency);
        self.max_ui_latency_ms = self.max_ui_latency_ms.max(latency);
        self.avg_ui_latency_ms =
            (latency - self.avg_ui_latency_ms).mul_add(0.1, self.avg_ui_latency_ms);
    }

    /// Resets all accumulated UI frame latency tracking statistics.
    pub const fn reset_ui_latency(&mut self) {
        self.min_ui_latency_ms = f32::MAX;
        self.max_ui_latency_ms = 0.0;
        self.avg_ui_latency_ms = 0.0;
    }
}
