//! # `ntd-output`
//!
//! Injects what the engine decided into the operating system, as virtual input events. It
//! implements the [`OutputSink`] port of `ntd-core` for the real system: `windows.rs` calls
//! `SendInput`, `linux.rs` creates virtual devices through `/dev/uinput`, and `macos.rs` and
//! `unsupported.rs` say [`OutputError::Unsupported`] and still compile. What each system can do is
//! told by [`OutputSink::capabilities`].
//!
//! # Design Decision: Synchronous Injection (Deliberately Not Decoupled)
//! Injector calls happen inline on the `TIME_CRITICAL`/`nice -11` polling thread, not on a
//! separate thread behind a channel. This is a known, intentional tradeoff, not an oversight.
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

pub use ntd_core::output::{OutputCapabilities, OutputSink};

use ntd_core::input::{Key, MouseButton};

/// Why the output could not be created.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputError {
    /// The system refuses to let the process inject input (Linux: `/dev/uinput`), with the steps
    /// that fix it.
    #[error("{0}")]
    PermissionDenied(String),
    /// This system has no output yet.
    #[error("not supported on this platform")]
    Unsupported,
    /// Any other failure, with its own message.
    #[error("{0}")]
    Other(String),
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::Injector;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::Injector;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::Injector;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub use unsupported::Injector;

impl OutputSink for Injector {
    fn capabilities(&self) -> OutputCapabilities {
        Self::CAPABILITIES
    }

    fn set_proximity(&mut self, in_proximity: bool) {
        Self::set_proximity(self, in_proximity);
    }

    fn move_absolute(
        &mut self,
        target_x: f32,
        target_y: f32,
        u: f32,
        v: f32,
        pressure: i32,
        tilt_x: i32,
        tilt_y: i32,
    ) {
        Self::move_absolute(self, target_x, target_y, u, v, pressure, tilt_x, tilt_y);
    }

    fn move_relative(&mut self, dx: f32, dy: f32) {
        Self::move_relative(self, dx, dy);
    }

    fn set_left_button(&mut self, is_down: bool) {
        Self::set_left_button(self, is_down);
    }

    fn set_mouse_button(&mut self, button: MouseButton, is_down: bool) {
        Self::set_mouse_button(self, button, is_down);
    }

    fn set_key(&mut self, key: Key, is_down: bool) {
        Self::set_key(self, key, is_down);
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;

    /// Only the calls that cannot reach the operating system: these tests must never move the
    /// developer's cursor or click.
    #[cfg(windows)]
    #[test]
    fn the_injector_forwards_the_calls_that_need_no_os_event() {
        let mut injector = Injector::try_new().unwrap();
        let sink: &mut dyn OutputSink = &mut injector;
        sink.set_proximity(true);
        // Less than a pixel: accumulated, not injected.
        sink.move_relative(0.25, 0.25);
        // The button is not held: nothing to release.
        sink.set_left_button(false);
    }

    #[test]
    fn an_output_that_does_nothing_offers_nothing() {
        let none = OutputCapabilities::NONE;
        assert!(!(none.absolute_pointer || none.relative_pointer || none.pressure || none.tilt));
    }

    #[test]
    fn errors_read_as_sentences() {
        assert_eq!(
            OutputError::Unsupported.to_string(),
            "not supported on this platform"
        );
        assert_eq!(
            OutputError::PermissionDenied("help".into()).to_string(),
            "help"
        );
    }

    #[test]
    fn the_sink_is_object_safe() {
        struct Nothing;
        impl OutputSink for Nothing {
            fn capabilities(&self) -> OutputCapabilities {
                OutputCapabilities::NONE
            }
            fn set_proximity(&mut self, _in_proximity: bool) {}
            fn move_absolute(
                &mut self,
                _target_x: f32,
                _target_y: f32,
                _u: f32,
                _v: f32,
                _pressure: i32,
                _tilt_x: i32,
                _tilt_y: i32,
            ) {
            }
            fn move_relative(&mut self, _dx: f32, _dy: f32) {}
            fn set_left_button(&mut self, _is_down: bool) {}
            fn set_mouse_button(&mut self, _button: MouseButton, _is_down: bool) {}
            fn set_key(&mut self, _key: Key, _is_down: bool) {}
        }
        let mut sink: Box<dyn OutputSink> = Box::new(Nothing);
        sink.set_proximity(false);
        sink.move_absolute(1.0, 2.0, 0.1, 0.2, 3, 4, 5);
        sink.move_relative(1.0, 2.0);
        sink.set_left_button(true);
        sink.set_mouse_button(MouseButton::Right, true);
        sink.set_key(Key::A, true);
    }
}
