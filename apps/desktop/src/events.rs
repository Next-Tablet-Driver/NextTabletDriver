//! Events the backend sends to the frontend. Each struct is the payload of one event; the event
//! name is the struct name in kebab-case (`TabletEvent` is `tablet-event`).

use serde::{Deserialize, Serialize};

/// One pen packet, about 60 times per second while a window is visible.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TabletEvent {
    /// Pen status as text, for example `Hover` or `Contact`.
    pub status: String,
    /// Horizontal position, in raw units of the tablet sensor.
    pub x: u32,
    /// Vertical position, in raw units of the tablet sensor.
    pub y: u32,
    /// Raw pressure, from 0 up to the maximum pressure of the tablet.
    pub pressure: u16,
    /// Distance of the pen above the surface, as the tablet reports it.
    pub hover_distance: u8,
    /// Horizontal tilt of the pen, in degrees.
    pub tilt_x: i8,
    /// Vertical tilt of the pen, in degrees.
    pub tilt_y: i8,
}

/// A new log line. The frontend subscribes to it; the backend does not send it yet (the console
/// reads `get_logs` instead).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct LogEvent {
    /// Formatted timestamp of the log line.
    pub time: String,
    /// Level of the log line: `INFO`, `DEBUG`, `WARN` or `ERROR`.
    pub level: String,
    /// Module or subsystem that wrote the log line.
    pub group: String,
    /// Text of the log line.
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
