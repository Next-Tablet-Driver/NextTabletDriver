//! # The `ntd-core` crate
//!
//! The types and the maths that every other crate of `NextTabletDriver` shares: button bindings,
//! the coordinate transforms of the pipeline and the mapping configuration. This crate does no
//! input or output, starts no thread and has no `unsafe` code.

/// Configuration models shared by the engine and the interface.
pub mod config;
pub mod input;
/// Maths of the pipeline: geometry and coordinate transforms.
pub mod math;
/// Helpers for poisoned locks.
pub mod sync;
