//! # Tablet Drivers and Parsing
//!
//! This module provides the infrastructure for hardware abstraction. It handles detection
//! (identifying supported HID devices on the USB bus), initialization (sending
//! vendor-specific "magic" packets to enable digitizer mode), and parsing (converting raw
//! byte arrays from various protocols into a unified format).
//!
//! Tablet configurations are sourced from the [OpenTabletDriver](https://github.com/OpenTabletDriver/OpenTabletDriver)
//! project via a git submodule at `tablets/OpenTabletDriver`, kept separate from this
//! crate's MIT-licensed sources since `OpenTabletDriver` is LGPLv3-licensed. To add support
//! for a new tablet, contribute the configuration upstream to `OpenTabletDriver`; it will be
//! picked up here on the next submodule update.

pub mod config;
pub mod config_loader;
pub mod detection;
pub mod generic;
pub mod models;
pub mod parsers;

pub use config::TabletConfiguration;
pub use config_loader::load_configurations;
pub use detection::detect_tablet;
pub use generic::GenericNextTabletDriver;
pub use models::{DriverStats, TabletData, TabletStatus};

/// The trait that all tablet-specific driver implementations must satisfy.
///
/// It provides the interface for the Engine to query hardware limits and
/// decode incoming USB data.
pub trait NextTabletDriver {
    /// Returns the marketing name of the tablet.
    fn get_name(&self) -> &str;
    /// Returns hardware resolution and max pressure: `(MaxX, MaxY, MaxPressure)`.
    fn get_specs(&self) -> (f32, f32, f32);
    /// Returns physical tablet size in millimeters: `(Width, Height)`.
    fn get_physical_specs(&self) -> (f32, f32);
    /// Returns the USB identity of the device: `(VendorID, ProductID)`.
    fn get_vid_pid(&self) -> (u16, u16);
    /// Attempts to parse a raw USB packet into standard [`TabletData`].
    /// Returns `None` if the packet is malformed or empty.
    fn parse(&self, data: &[u8]) -> Option<TabletData>;
}
