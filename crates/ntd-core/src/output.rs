//! The port through which the engine injects input into the operating system.

use crate::input::{Key, MouseButton};

/// What the polling loop needs from an OS input injector.
///
/// The real injector talks to the operating system. Having the loop depend on this trait instead
/// makes the per-packet code runnable without injecting anything: the benchmarks and the tests
/// use a fake from `ntd-test-support`.
pub trait OutputSink {
    /// Updates stylus proximity.
    fn set_proximity(&mut self, in_proximity: bool);
    /// Moves to an absolute position (`Absolute` mode).
    #[allow(
        clippy::too_many_arguments,
        reason = "the position, pressure and tilt fields of one pen sample, as every sink receives \
            them"
    )]
    fn move_absolute(
        &mut self,
        target_x: f32,
        target_y: f32,
        u: f32,
        v: f32,
        pressure: i32,
        tilt_x: i32,
        tilt_y: i32,
    );
    /// Moves by a delta (`Relative` mode).
    fn move_relative(&mut self, dx: f32, dy: f32);
    /// Presses or releases the tip button.
    fn set_left_button(&mut self, is_down: bool);
    /// Presses or releases a mouse button on behalf of a button binding.
    fn set_mouse_button(&mut self, button: MouseButton, is_down: bool);
    /// Presses or releases a keyboard key on behalf of a button binding.
    fn set_key(&mut self, key: Key, is_down: bool);
}
