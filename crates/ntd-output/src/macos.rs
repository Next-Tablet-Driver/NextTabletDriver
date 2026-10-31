//! macOS: not implemented in 3.0. [`Injector::try_new`] says [`OutputError::Unsupported`], and the
//! crate still compiles for the target.

use crate::OutputError;
use ntd_core::input::{Key, MouseButton};
use ntd_core::output::OutputCapabilities;

/// The output of this system: none yet, so it cannot be created.
pub struct Injector {
    _private: (),
}

#[allow(
    clippy::unused_self,
    clippy::missing_const_for_fn,
    clippy::needless_pass_by_ref_mut,
    reason = "the same methods as the systems that inject, which an Injector of this system can \
        never reach since it cannot be created"
)]
impl Injector {
    pub(crate) const CAPABILITIES: OutputCapabilities = OutputCapabilities::NONE;

    /// Always fails: there is no output for this system yet.
    ///
    /// # Errors
    /// Always [`OutputError::Unsupported`].
    pub fn try_new() -> Result<Self, OutputError> {
        Err(OutputError::Unsupported)
    }

    pub(crate) fn set_proximity(&mut self, _in_proximity: bool) {}

    #[allow(
        clippy::too_many_arguments,
        reason = "implements the `OutputSink` method of the same signature"
    )]
    pub(crate) fn move_absolute(
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

    pub(crate) fn move_relative(&mut self, _dx: f32, _dy: f32) {}

    pub(crate) fn set_left_button(&mut self, _is_down: bool) {}

    pub(crate) fn set_mouse_button(&mut self, _button: MouseButton, _is_down: bool) {}

    pub(crate) fn set_key(&mut self, _key: Key, _is_down: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_system_has_no_output() {
        assert!(matches!(Injector::try_new(), Err(OutputError::Unsupported)));
    }
}
