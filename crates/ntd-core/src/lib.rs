//! # The `ntd-core` crate
//!
//! The types and the maths that every other crate of `NextTabletDriver` shares: button bindings,
//! the coordinate transforms of the pipeline and the mapping configuration. This crate does no
//! input or output (reading a clock is its only contact with the OS), starts no thread and has no
//! `unsafe` code.

pub mod clock;
/// Configuration models shared by the engine and the interface.
pub mod config;
/// The port to HID devices: what the engine needs, implemented by the crates that do the I/O.
pub mod hid;
pub mod input;
/// The languages of the interface.
pub mod locale;
/// Maths of the pipeline: geometry and coordinate transforms.
pub mod math;
/// The port to the operating system's input injection.
pub mod output;
/// Helpers for poisoned locks.
pub mod sync;
