//! # Input Processing Engine
//!
//! This module forms the high-performance core of `NextTabletDriver`.
//! It handles the background lifecycle of tablet devices, runs the mapping pipeline
//! to transform raw HID data into screen coordinates, and injects the resulting
//! virtual events into the operating system.

#[cfg(feature = "desktop")]
pub mod injector;
pub mod interop;
pub mod pipeline;
pub mod plugins;
pub mod state;
#[cfg(feature = "desktop")]
pub mod tablet_manager;
