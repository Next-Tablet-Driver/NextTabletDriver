//! # Button routing
//!
//! Turns the pen buttons and the tablet keys reported by the parsers into the mouse buttons
//! and keys the user bound them to (`pen_button_bindings`, `aux_button_bindings`).
//!
//! The router only looks at what changed since the previous report, so a report with no
//! button change costs one comparison. A binding is read from the settings when its button
//! is pressed and remembered until the button is released, so editing a binding while the
//! button is held cannot leave the old key stuck down.
//!
//! Two bindings can share a key (`Ctrl+Z` and `Ctrl+Y` both use `Ctrl`). Each key and mouse
//! button is counted, and the operating system only sees the first press and the last
//! release.

use ntd_core::config::models::MappingConfig;
use ntd_core::input::{Binding, Key, MouseButton};
use ntd_core::output::OutputSink;
use ntd_tablets::{TabletData, TabletStatus};

/// How many buttons of one kind the router follows: `TabletData::buttons` is a byte.
const BUTTONS_PER_GROUP: usize = 8;

/// The state of one set of buttons (the pen's, or the tablet's keys).
#[derive(Default)]
struct ButtonGroup {
    /// Bit `n` is set while physical button `n` is held.
    held: u8,
    /// The binding that was applied when each held button was pressed.
    applied: [Binding; BUTTONS_PER_GROUP],
}

/// Follows the pen buttons and the tablet keys and presses or releases what they are bound to.
#[derive(Default)]
pub(super) struct ButtonRouter {
    pen: ButtonGroup,
    aux: ButtonGroup,
    key_refs: KeyRefs,
}

/// How many held bindings use each key and each mouse button.
struct KeyRefs {
    keys: [u8; Key::COUNT],
    mouse: [u8; MouseButton::COUNT],
}

impl Default for KeyRefs {
    fn default() -> Self {
        Self {
            keys: [0; Key::COUNT],
            mouse: [0; MouseButton::COUNT],
        }
    }
}

impl ButtonRouter {
    /// A router with nothing held.
    #[must_use]
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Applies the button state carried by one report.
    pub(super) fn update<I: OutputSink>(
        &mut self,
        sink: &mut I,
        data: &TabletData,
        config: &MappingConfig,
    ) {
        if !data.is_connected {
            self.release_pen(sink);
            return;
        }
        match data.status {
            TabletStatus::Contact | TabletStatus::Hover | TabletStatus::Active => {
                Self::apply(
                    &mut self.pen,
                    &mut self.key_refs,
                    sink,
                    data.buttons,
                    &config.pen_button_bindings,
                );
            }
            TabletStatus::Aux => {
                Self::apply(
                    &mut self.aux,
                    &mut self.key_refs,
                    sink,
                    data.buttons,
                    &config.aux_button_bindings,
                );
            }
            TabletStatus::Disconnected | TabletStatus::OutOfRange => self.release_pen(sink),
            _ => {}
        }
    }

    /// Releases everything the pen buttons hold. Called when the pen leaves the tablet, where
    /// a button can no longer be reported as released.
    pub(super) fn release_pen<I: OutputSink>(&mut self, sink: &mut I) {
        Self::apply(&mut self.pen, &mut self.key_refs, sink, 0, &[]);
    }

    /// Releases everything that is held, pen buttons and tablet keys. Called when the tablet
    /// goes away or the engine stops, so no key stays pressed.
    pub(super) fn release_all<I: OutputSink>(&mut self, sink: &mut I) {
        Self::apply(&mut self.pen, &mut self.key_refs, sink, 0, &[]);
        Self::apply(&mut self.aux, &mut self.key_refs, sink, 0, &[]);
    }

    fn apply<I: OutputSink>(
        group: &mut ButtonGroup,
        refs: &mut KeyRefs,
        sink: &mut I,
        mask: u8,
        bindings: &[String],
    ) {
        let changed = group.held ^ mask;
        if changed == 0 {
            return;
        }
        for index in 0..BUTTONS_PER_GROUP {
            let bit = 1u8 << index;
            if changed & bit == 0 {
                continue;
            }
            let Some(slot) = group.applied.get_mut(index) else {
                continue;
            };
            if mask & bit == 0 {
                release(refs, sink, &std::mem::take(slot));
            } else {
                let binding = bindings
                    .get(index)
                    .map_or(Binding::None, |text| parse_binding(index, text));
                press(refs, sink, &binding);
                *slot = binding;
            }
        }
        group.held = mask;
    }
}

/// Reads a binding from the settings; a text that cannot be understood does nothing, and is
/// reported once per press so a typo can be found in the log.
fn parse_binding(index: usize, text: &str) -> Binding {
    text.parse().unwrap_or_else(|error| {
        log::warn!(
            target: "Buttons",
            "Ignoring the binding '{text}' of button {}: {error}",
            index + 1
        );
        Binding::None
    })
}

fn press<I: OutputSink>(refs: &mut KeyRefs, sink: &mut I, binding: &Binding) {
    match binding {
        Binding::None => {}
        Binding::Mouse(button) => {
            if let Some(count) = refs.mouse.get_mut(button.index()) {
                *count = count.saturating_add(1);
                if *count == 1 {
                    sink.set_mouse_button(*button, true);
                }
            }
        }
        Binding::Chord(keys) => {
            for &key in keys {
                if let Some(count) = refs.keys.get_mut(key as usize) {
                    *count = count.saturating_add(1);
                    if *count == 1 {
                        sink.set_key(key, true);
                    }
                }
            }
        }
    }
}

fn release<I: OutputSink>(refs: &mut KeyRefs, sink: &mut I, binding: &Binding) {
    match binding {
        Binding::None => {}
        Binding::Mouse(button) => {
            if let Some(count) = refs.mouse.get_mut(button.index()) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    sink.set_mouse_button(*button, false);
                }
            }
        }
        Binding::Chord(keys) => {
            // Reverse order, so a modifier is released after the key it modifies.
            for &key in keys.iter().rev() {
                if let Some(count) = refs.keys.get_mut(key as usize) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        sink.set_key(key, false);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the router asked the operating system to do, in order.
    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Mouse(MouseButton, bool),
        Key(Key, bool),
    }

    #[derive(Default)]
    struct Recorder {
        events: Vec<Event>,
    }

    impl OutputSink for Recorder {
        fn set_proximity(&mut self, _in_proximity: bool) {}
        fn move_absolute(
            &mut self,
            _x: f32,
            _y: f32,
            _u: f32,
            _v: f32,
            _p: i32,
            _tx: i32,
            _ty: i32,
        ) {
        }
        fn move_relative(&mut self, _dx: f32, _dy: f32) {}
        fn set_left_button(&mut self, _is_down: bool) {}
        fn set_mouse_button(&mut self, button: MouseButton, is_down: bool) {
            self.events.push(Event::Mouse(button, is_down));
        }
        fn set_key(&mut self, key: Key, is_down: bool) {
            self.events.push(Event::Key(key, is_down));
        }
    }

    fn config(pen: &[&str], aux: &[&str]) -> MappingConfig {
        MappingConfig {
            pen_button_bindings: pen.iter().map(ToString::to_string).collect(),
            aux_button_bindings: aux.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    fn report(status: TabletStatus, buttons: u8) -> TabletData {
        TabletData {
            status,
            buttons,
            is_connected: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_pressed_pen_button_presses_its_mouse_button_until_it_is_released() {
        let config = config(&["Right Click", "None"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 0b01), &config);
        router.update(&mut sink, &report(TabletStatus::Hover, 0b01), &config);
        router.update(&mut sink, &report(TabletStatus::Hover, 0b00), &config);

        assert_eq!(
            sink.events,
            [
                Event::Mouse(MouseButton::Right, true),
                Event::Mouse(MouseButton::Right, false)
            ]
        );
    }

    #[test]
    fn the_second_pen_button_uses_the_second_binding() {
        let config = config(&["None", "Back"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Contact, 0b10), &config);

        assert_eq!(sink.events, [Event::Mouse(MouseButton::Back, true)]);
    }

    #[test]
    fn a_chord_is_pressed_in_order_and_released_in_reverse() {
        let config = config(&["Ctrl+Shift+Z"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &report(TabletStatus::Hover, 0), &config);

        assert_eq!(
            sink.events,
            [
                Event::Key(Key::Ctrl, true),
                Event::Key(Key::Shift, true),
                Event::Key(Key::Z, true),
                Event::Key(Key::Z, false),
                Event::Key(Key::Shift, false),
                Event::Key(Key::Ctrl, false),
            ]
        );
    }

    #[test]
    fn tablet_keys_use_their_own_bindings_not_the_pen_ones() {
        let config = config(&["Right Click"], &["Space", "Ctrl+Z"]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Aux, 0b01), &config);

        assert_eq!(sink.events, [Event::Key(Key::Space, true)]);
    }

    #[test]
    fn a_pen_button_and_a_tablet_key_do_not_disturb_each_other() {
        let config = config(&["Right Click"], &["Space"]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &report(TabletStatus::Aux, 1), &config);
        // The pen keeps hovering with its button held: the key stays down.
        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &report(TabletStatus::Aux, 0), &config);

        assert_eq!(
            sink.events,
            [
                Event::Mouse(MouseButton::Right, true),
                Event::Key(Key::Space, true),
                Event::Key(Key::Space, false),
            ]
        );
    }

    #[test]
    fn a_shared_modifier_is_released_only_after_the_last_binding_using_it() {
        let config = config(&["Ctrl+Z", "Ctrl+Y"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 0b01), &config);
        router.update(&mut sink, &report(TabletStatus::Hover, 0b11), &config);
        router.update(&mut sink, &report(TabletStatus::Hover, 0b10), &config);
        router.update(&mut sink, &report(TabletStatus::Hover, 0b00), &config);

        assert_eq!(
            sink.events,
            [
                Event::Key(Key::Ctrl, true),
                Event::Key(Key::Z, true),
                Event::Key(Key::Y, true),
                Event::Key(Key::Z, false),
                Event::Key(Key::Y, false),
                Event::Key(Key::Ctrl, false),
            ]
        );
    }

    #[test]
    fn editing_a_binding_while_its_button_is_held_does_not_leave_a_key_stuck() {
        let before = config(&["A"], &[]);
        let after = config(&["B"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &before);
        router.update(&mut sink, &report(TabletStatus::Hover, 0), &after);

        assert_eq!(
            sink.events,
            [Event::Key(Key::A, true), Event::Key(Key::A, false)]
        );
    }

    #[test]
    fn a_button_without_a_binding_or_with_none_does_nothing() {
        let config = config(&["None"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 0b11), &config);
        router.update(&mut sink, &report(TabletStatus::Aux, 0b1), &config);

        assert!(sink.events.is_empty());
    }

    #[test]
    fn a_binding_that_cannot_be_read_does_nothing() {
        crate::test_support::evaluate_log_arguments();
        let config = config(&["Ctrl+Banana"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &report(TabletStatus::Hover, 0), &config);

        assert!(sink.events.is_empty());
    }

    #[test]
    fn leaving_the_tablet_releases_the_pen_buttons_but_not_the_tablet_keys() {
        let config = config(&["Right Click"], &["Space"]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &report(TabletStatus::Aux, 1), &config);
        router.update(&mut sink, &report(TabletStatus::OutOfRange, 0), &config);

        assert_eq!(
            sink.events,
            [
                Event::Mouse(MouseButton::Right, true),
                Event::Key(Key::Space, true),
                Event::Mouse(MouseButton::Right, false),
            ]
        );
    }

    #[test]
    fn a_disconnected_tablet_releases_the_pen_buttons() {
        let config = config(&["Space"], &[]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &TabletData::default(), &config);

        assert_eq!(
            sink.events,
            [Event::Key(Key::Space, true), Event::Key(Key::Space, false)]
        );
    }

    #[test]
    fn release_all_lets_go_of_everything() {
        let config = config(&["Right Click"], &["Ctrl+Z"]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &report(TabletStatus::Aux, 1), &config);
        sink.events.clear();
        router.release_all(&mut sink);

        assert_eq!(
            sink.events,
            [
                Event::Mouse(MouseButton::Right, false),
                Event::Key(Key::Z, false),
                Event::Key(Key::Ctrl, false),
            ]
        );
        // A second call has nothing left to release.
        sink.events.clear();
        router.release_all(&mut sink);
        assert!(sink.events.is_empty());
    }

    #[test]
    fn reports_that_carry_no_button_state_are_ignored() {
        let config = config(&["Space"], &["Space"]);
        let (mut router, mut sink) = (ButtonRouter::new(), Recorder::default());

        router.update(&mut sink, &report(TabletStatus::Hover, 1), &config);
        router.update(&mut sink, &report(TabletStatus::Tool, 0), &config);

        assert_eq!(sink.events, [Event::Key(Key::Space, true)]);
    }
}
