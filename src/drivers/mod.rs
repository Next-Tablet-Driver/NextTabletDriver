//! # Tablet detection
//!
//! Finds the connected tablet among the HID devices and initializes it (sends the vendor-specific
//! "magic" packets that enable digitizer mode). What the program knows about tablets (the
//! definitions, the parsers and the packet types) lives in the `ntd-tablets` crate.

/// Loading of the tablet definitions, including the local `tablets` folder.
pub mod config_loader;
/// Detection of the connected tablet among the HID devices.
pub mod detection;
#[cfg(windows)]
pub mod hid_caps;

pub use config_loader::load_configurations;
pub use detection::detect_tablet;
