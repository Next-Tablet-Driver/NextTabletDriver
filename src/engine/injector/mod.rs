//! # OS Event Injection
//!
//! This module abstracts the interaction with the operating system's input APIs.
//! It takes normalized screen coordinates and button states from the pipeline
//! and injects them as virtual input events.
//!
//! # Platform Specifics
//! - **Windows**: Uses `enigo` + `windows-sys` for mouse simulation via `SendInput`.
//! - **Linux**: Creates a virtual tablet device via `/dev/uinput` (kernel module)
//!   using the `evdev` crate. This approach is universally compatible with
//!   X11, Wayland, and `XWayland` — the kernel sees it as real hardware.

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::Injector;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::Injector;
