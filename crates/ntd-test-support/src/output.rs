use ntd_core::input::{Key, MouseButton};
use ntd_core::output::{OutputCapabilities, OutputSink};

/// One call of the engine to an [`OutputSink`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputEvent {
    /// `set_proximity`.
    Proximity(bool),
    /// `move_absolute`.
    MoveAbsolute {
        /// Target X, in pixels.
        x: f32,
        /// Target Y, in pixels.
        y: f32,
        /// Horizontal position in the active area, 0 to 1.
        u: f32,
        /// Vertical position in the active area, 0 to 1.
        v: f32,
        /// Pressure, 0 to 8191.
        pressure: i32,
        /// Horizontal tilt, in degrees.
        tilt_x: i32,
        /// Vertical tilt, in degrees.
        tilt_y: i32,
    },
    /// `move_relative`.
    MoveRelative {
        /// Horizontal movement.
        dx: f32,
        /// Vertical movement.
        dy: f32,
    },
    /// `set_left_button`.
    LeftButton(bool),
    /// `set_mouse_button`.
    MouseButton(MouseButton, bool),
    /// `set_key`.
    Key(Key, bool),
}

/// An [`OutputSink`] that injects nothing and remembers every call, in order.
#[derive(Debug, Clone)]
pub struct RecordingSink {
    /// What the engine asked for, oldest first.
    pub events: Vec<OutputEvent>,
    /// What this sink says it can do; everything unless the test narrows it.
    pub capabilities: OutputCapabilities,
}

impl Default for RecordingSink {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordingSink {
    /// An empty recording.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: Vec::new(),
            capabilities: OutputCapabilities {
                absolute_pointer: true,
                relative_pointer: true,
                pressure: true,
                tilt: true,
            },
        }
    }

    /// How many calls moved the pointer, in either mode.
    #[must_use]
    pub fn moves(&self) -> usize {
        self.events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    OutputEvent::MoveAbsolute { .. } | OutputEvent::MoveRelative { .. }
                )
            })
            .count()
    }

    /// The last call, if any.
    #[must_use]
    pub fn last(&self) -> Option<&OutputEvent> {
        self.events.last()
    }
}

impl OutputSink for RecordingSink {
    fn capabilities(&self) -> OutputCapabilities {
        self.capabilities
    }

    fn set_proximity(&mut self, in_proximity: bool) {
        self.events.push(OutputEvent::Proximity(in_proximity));
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
        self.events.push(OutputEvent::MoveAbsolute {
            x: target_x,
            y: target_y,
            u,
            v,
            pressure,
            tilt_x,
            tilt_y,
        });
    }

    fn move_relative(&mut self, dx: f32, dy: f32) {
        self.events.push(OutputEvent::MoveRelative { dx, dy });
    }

    fn set_left_button(&mut self, is_down: bool) {
        self.events.push(OutputEvent::LeftButton(is_down));
    }

    fn set_mouse_button(&mut self, button: MouseButton, is_down: bool) {
        self.events.push(OutputEvent::MouseButton(button, is_down));
    }

    fn set_key(&mut self, key: Key, is_down: bool) {
        self.events.push(OutputEvent::Key(key, is_down));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_sink_offers_everything_unless_the_test_narrows_it() {
        let mut sink = RecordingSink::new();
        assert!(sink.capabilities().pressure && sink.capabilities().tilt);
        sink.capabilities = OutputCapabilities::NONE;
        let port: &dyn OutputSink = &sink;
        assert_eq!(port.capabilities(), OutputCapabilities::NONE);
    }

    #[test]
    fn every_call_is_recorded_in_order() {
        let mut sink = RecordingSink::new();
        let port: &mut dyn OutputSink = &mut sink;
        port.set_proximity(true);
        port.move_absolute(10.0, 20.0, 0.1, 0.2, 300, -4, 5);
        port.move_relative(1.5, -2.5);
        port.set_left_button(true);
        port.set_mouse_button(MouseButton::Right, true);
        port.set_key(Key::A, false);

        assert_eq!(sink.events.len(), 6);
        assert_eq!(sink.events.first(), Some(&OutputEvent::Proximity(true)));
        assert_eq!(
            sink.events.get(1),
            Some(&OutputEvent::MoveAbsolute {
                x: 10.0,
                y: 20.0,
                u: 0.1,
                v: 0.2,
                pressure: 300,
                tilt_x: -4,
                tilt_y: 5,
            })
        );
        assert_eq!(sink.last(), Some(&OutputEvent::Key(Key::A, false)));
        assert_eq!(sink.moves(), 2);
    }
}
