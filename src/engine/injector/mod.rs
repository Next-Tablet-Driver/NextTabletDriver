//! # OS Event Injection
//!
//! This module abstracts the interaction with the operating system's input APIs.
//! It takes normalized screen coordinates and button states from the pipeline
//! and injects them as virtual input events.
//!
//! # Platform Specifics
//! - **Windows**: Uses `windows-sys` to call `SendInput` directly for mouse simulation.
//! - **Linux**: Creates a virtual tablet device via `/dev/uinput` (kernel module)
//!   using the `evdev` crate. This approach is universally compatible with
//!   X11, Wayland, and `XWayland` - the kernel sees it as real hardware.
//!
//! # Design Decision: Synchronous Injection (Deliberately Not Decoupled)
//! Injector calls happen inline on the `TIME_CRITICAL`/`nice -11` polling thread
//! (see `pipeline::process`), not on a separate thread behind a channel. This is
//! a known, intentional tradeoff, not an oversight.
//!
//! - **The theoretical risk**: `SendInput` on Windows walks the full OS input
//!   stack, including any third-party low-level mouse hooks (`WH_MOUSE_LL`),
//!   such as macro tools, overlays, or capture software. A slow hook could in
//!   theory delay the polling thread and cause a missed or late HID read.
//!   `uinput` writes on Linux are less exposed to this but are still a
//!   blocking syscall on the hot path.
//! - **Why it's not being fixed**: decoupling this into a dedicated injection
//!   thread and channel is not a simple change. Position updates can be
//!   coalesced to "latest wins", but button press/release events cannot be
//!   dropped or reordered relative to position without producing stuck drags
//!   or phantom clicks. That means two different transport semantics, plus new
//!   thread lifecycle and error handling, in exchange for a benefit that has
//!   never been measured or reported. The expected real-world gain does not
//!   justify the added concurrency risk.
//! - **When to revisit**: only if there's concrete evidence, profiling data or
//!   user reports, linking actual stutters to slow injection calls, not as a
//!   speculative optimization.

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::Injector;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::Injector;
