//! Events the backend sends to the frontend. Each struct is the payload of one event; the event
//! name is the struct name in kebab-case (`TabletEvent` is `tablet-event`).

use serde::{Deserialize, Serialize};

/// One pen packet, about 60 times per second while a window is visible.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TabletEvent {
    pub status: String,
    pub x: u32,
    pub y: u32,
    pub pressure: u16,
    pub hover_distance: u8,
    pub tilt_x: i8,
    pub tilt_y: i8,
}

/// A new log line. The frontend subscribes to it; the backend does not send it yet (the console
/// reads `get_logs` instead).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct LogEvent {
    pub time: String,
    pub level: String,
    pub group: String,
    pub message: String,
}

/// The connected tablet changed (plugged, unplugged or swapped).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct DeviceChanged;

/// A profile was loaded from the tray menu; the frontend must reload the configuration.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct ProfileLoaded;

/// The tray menu asked to show the updates page.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct NavigateToUpdates;
