use crate::core::config::models::MappingConfig;
use std::path::PathBuf;
use std::time::Instant;

/// Represents the currently active tab in the main application window.
#[derive(PartialEq, Clone, Copy)]
pub enum AppTab {
    Output,
    Filters,
    PenSettings,
    Console,
    Settings,
    Release,
}

/// Tracks the state of the currently active user profile.
pub struct ProfileState {
    pub name: String,
    pub path: Option<PathBuf>,
    pub last_saved: MappingConfig,
}

impl ProfileState {
    pub fn is_dirty(&self, current: &MappingConfig) -> bool {
        *current != self.last_saved
    }

    pub fn display_name(&self, current: &MappingConfig) -> String {
        let base = if self.path.is_some() {
            &self.name
        } else {
            "Unsaved Session"
        };

        if self.is_dirty(current) {
            format!("*{}", base)
        } else {
            base.to_string()
        }
    }

    pub fn mark_saved(&mut self, config: &MappingConfig) {
        self.last_saved = config.clone();
    }
}

/// Severity level for UI toast notifications.
#[derive(Clone, Copy, PartialEq)]
pub enum ToastLevel {
    Info,
    Warning,
    Error,
}

/// A transient notification displayed in the UI overlay.
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
}

/// Encapsulates performance metrics and latency tracking.
pub struct Metrics {
    pub displayed_hz: f32,
    pub last_hz_update: Instant,
    pub last_packet_count: u32,
    pub ui_latency_ms: f32,
    pub min_ui_latency_ms: f32,
    pub max_ui_latency_ms: f32,
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
    pub fn update_hz(&mut self, current_packets: u32) {
        let elapsed = self.last_hz_update.elapsed();
        if elapsed >= std::time::Duration::from_millis(200) {
            let delta = current_packets.saturating_sub(self.last_packet_count);
            let hz = delta as f32 / elapsed.as_secs_f32();
            self.displayed_hz += (hz - self.displayed_hz) * 0.3;
            self.last_packet_count = current_packets;
            self.last_hz_update = Instant::now();
        }
    }

    pub fn update_latency(&mut self, latency: f32) {
        self.ui_latency_ms = latency;
        self.min_ui_latency_ms = self.min_ui_latency_ms.min(latency);
        self.max_ui_latency_ms = self.max_ui_latency_ms.max(latency);
        self.avg_ui_latency_ms += (latency - self.avg_ui_latency_ms) * 0.1;
    }

    pub fn reset_ui_latency(&mut self) {
        self.min_ui_latency_ms = f32::MAX;
        self.max_ui_latency_ms = 0.0;
        self.avg_ui_latency_ms = 0.0;
    }
}
