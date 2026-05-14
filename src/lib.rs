//! # NextTabletDriver (NextTD) Core Library
//!
//! NextTabletDriver is a high-performance, cross-platform tablet driver designed for
//! drawing and rhythm games (like osu!). It focuses on low latency, modularity,
//! and broad hardware compatibility.
//!
//! ## Architecture Overview
//!
//! The driver operates as a pipeline with the following stages:
//!
//! 1.  **HID Detection & Polling** ([`drivers`]): Scans the USB bus for supported devices
//!     using `hidapi` and initiates a high-frequency polling loop.
//! 2.  **Packet Parsing** ([`drivers::parsers`]): Decodes vendor-specific byte arrays
//!     into a standardized [`drivers::TabletData`] structure.
//! 3.  **Filter Pipeline** ([`filters`]): Applies optional processing such as
//!     antichatter (smoothing) or telemetry collection.
//! 4.  **Coordinate Transformation** ([`core::math`]): Maps physical tablet coordinates
//!     into normalized space, then projects them onto the target screen pixels.
//! 5.  **Event Injection** ([`engine::injector`]): Injects the final coordinates and
//!     button states into the operating system (uinput on Linux, SendInput on Windows).
//!
//! ## Threading Model
//!
//! NextTabletDriver uses a multi-threaded architecture to ensure UI responsiveness:
//! *   **Input Engine Thread**: Handles HID polling and the entire processing pipeline.
//! *   **GUI Thread**: Runs the `egui` interface for configuration and monitoring.
//! *   **WebSocket Thread**: Provides real-time data to external integrations.
//!
//! ## Key Modules
//! *   **[`app`]**: GUI application state and lifecycle management.
//! *   **[`core`]**: Core configuration models and mathematical transforms.
//! *   **[`drivers`]**: Protocol definitions and hardware-specific parsers.
//! *   **[`engine`]**: The execution core that links the UI, drivers, and OS.
//! *   **[`filters`]**: Digital signal processing for pen data.
//! *   **[`settings`]**: Persistence logic for user configurations.
//! *   **[`startup`]**: Platform-specific autostart registration.
//! *   **[`ui`]**: Graphical components and theming.

pub mod app;
pub mod core;
pub mod drivers;
pub mod engine;
pub mod filters;
pub mod logger;
pub mod settings;
pub mod startup;
pub mod ui;

/// Version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
