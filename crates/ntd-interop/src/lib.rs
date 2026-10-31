//! # `ntd-interop`
//!
//! How the desktop app and any number of SDK-embedded engines coexist without opening the same
//! HID device twice. Exactly one process at a time is the "HID owner" ([`lock`]); everyone else
//! reads the state the owner publishes ([`shm`]) and forwards config writes to it ([`command`]).
//!
//! The crate separates the **protocol** from the **transports**:
//!
//! - [`protocol`] is pure: the snapshot layout, the commands and their fixed-size encoding, and
//!   the names. It is the part that is fuzzed.
//! - [`shm`], [`command`] and [`lock`] move those bytes: a seqlock over shared memory (`windows.rs`
//!   and `linux.rs` map it; `macos.rs` and `unsupported.rs` map nothing), a local socket, and the
//!   machine-wide lock of `ntd-platform`.
//!
//! Used by both the desktop app and the SDK's embedded engine loop: neither side duplicates the
//! lock, the shared-memory layout or the command protocol.

pub mod command;
pub mod lock;
pub mod protocol;
pub mod shm;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as os;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as os;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
use unsupported as os;
