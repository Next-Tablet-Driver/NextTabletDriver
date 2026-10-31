//! # `NextTabletDriver` (`NextTD`) Core Library
//!
//! `NextTabletDriver` is a high-performance, cross-platform tablet driver designed for
//! drawing and rhythm games (like osu!). It focuses on low latency, modularity,
//! and broad hardware compatibility.
//!
//! ## Architecture Overview
//!
//! The driver operates as a pipeline with the following stages: [`ntd_hid`] scans the USB
//! bus for supported devices through `ntd-hid` and runs a high-frequency polling loop,
//! [`ntd_tablets::parsers`] decodes the vendor-specific byte arrays into a standardized
//! [`ntd_tablets::TabletData`] structure, [`ntd_pipeline`] applies optional processing such as
//! antichatter (smoothing) or telemetry collection, [`ntd_core::math`] maps physical tablet
//! coordinates into normalized space and projects them onto the target screen pixels, and
//! [`ntd_output`] injects the final coordinates and button states into the operating
//! system (uinput on Linux, `SendInput` on Windows).
//!
//! ## Threading Model
//!
//! `NextTabletDriver` uses a multi-threaded architecture to ensure UI responsiveness:
//! *   **Input Engine Thread**: Handles HID polling and the entire processing pipeline.
//! *   **UI Thread**: The Tauri webview, which talks to the engine through shared state.
//! *   **WebSocket Thread**: Provides real-time data to external integrations.
//!
//! ## Key Modules
//! *   **[`ntd_core`]**: Core configuration models and mathematical transforms (own crate).
//! *   **[`ntd_hid`]**: The HID backend and the detection of the connected tablet (own crate).
//! *   **[`ntd_state`]**: The state shared by the engine threads and the interface (own crate).
//! *   **[`engine`]**: The execution core that links the UI, drivers, and OS.
//! *   **[`ntd_pipeline`]**: The processing stages and the filter chain (own crate).
//! *   **[`i18n`]**: Internationalization support.
//! *   **[`logger`]**: Logging utilities.
//! *   **[`ntd_settings`]**: Persistence logic for user configurations (own crate).
//! *   **[`startup`]**: Platform-specific autostart registration.

/// Application layer: services, the state the interface reads, and telemetry.
pub mod application;
pub mod engine;
pub mod i18n;
/// Logging to a file and to the in-memory buffer behind the console.
pub mod logger;
/// Persistence of profiles, sessions, themes and preferences.
pub mod startup;

#[cfg(test)]
mod test_support;

/// Version.
pub const VERSION: &str = "3.0.0";
