//! # Inter-Process HID Arbitration
//!
//! Shared coordination mechanism letting the desktop app and any number of
//! SDK-embedded engine instances (games, plugins) coexist without opening the
//! same HID device twice. Exactly one process at a time is the "HID owner"
//! (see [`lock`]); everyone else falls back to reading state the owner
//! publishes and forwarding config writes to it (`shm`/`command`, added
//! alongside the rest of the arbitration mechanism).
//!
//! Used by both the desktop app (`engine::tablet_manager`) and the SDK's
//! embedded engine loop, which is why it lives in the root crate rather than
//! in `sdk/`. Neither side duplicates the lock, the shared-memory layout, or
//! the command protocol.

pub mod command;
pub mod lock;
pub mod shm;
