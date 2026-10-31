//! # `ntd-hid`
//!
//! The only crate of the workspace that depends on `hidapi`. It implements the HID port of
//! `ntd-core` ([`HidBackend`] and [`HidDevice`]) for the real hardware, so that the engine and the
//! tools can be tested with the fakes of `ntd-test-support` instead.
//!
//! [`Backend`] is the implementation of the current system: `windows.rs` and `linux.rs` use
//! `hidapi` (Windows also asks the OS for the static report length), `macos.rs` and
//! `unsupported.rs` say [`HidError::Unsupported`] and still compile. This crate delivers raw bytes
//! and never parses them.

pub use ntd_core::hid::{HidBackend, HidDevice, HidDeviceInfo, HidError, OpenOptions};

#[cfg(any(windows, target_os = "linux"))]
mod hidapi_backend;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::Backend;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::Backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::Backend;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub use unsupported::Backend;
