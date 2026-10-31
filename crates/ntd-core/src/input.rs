//! # Button bindings
//!
//! What a pen button or a tablet key can be mapped to, and the text format the user and the
//! configuration file use for it. This module has no operating system code: turning a
//! [`Binding`] into real input events is the job of the injector.
//!
//! # Format
//!
//! A binding is one of:
//! - `None` (also `Disabled`): nothing happens.
//! - a mouse button: `Left Click`, `Right Click`, `Middle Click`, `Back`, `Forward`.
//! - a key or a chord of up to [`MAX_CHORD_LEN`] keys joined by `+`: `Space`, `Ctrl+Z`,
//!   `Ctrl+Shift+S`. Keys are pressed in the order written and released in reverse order.
//!
//! Names are not case sensitive. The `OpenTabletDriver` spellings used by older settings files
//! are accepted too: `Mouse Button Binding: (Button: Left)` and `Key Binding: (Key: Space)`.
//! [`Binding`] always prints in the plain format above.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// Longest chord a binding can hold (for example `Ctrl+Shift+Alt+Z`).
pub const MAX_CHORD_LEN: usize = 4;

/// A mouse button a binding can press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Wheel button.
    Middle,
    /// "Back" side button (`XBUTTON1`).
    Back,
    /// "Forward" side button (`XBUTTON2`).
    Forward,
}

impl MouseButton {
    /// How many mouse buttons there are.
    pub const COUNT: usize = 5;

    /// Position of the button in a table of [`Self::COUNT`] entries.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Name used when the binding is printed.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Left => "Left Click",
            Self::Right => "Right Click",
            Self::Middle => "Middle Click",
            Self::Back => "Back",
            Self::Forward => "Forward",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match normalize(name).as_str() {
            // A bare `Left` or `Right` is the arrow key, not a mouse button.
            "leftclick" | "leftbutton" | "lmb" => Some(Self::Left),
            "rightclick" | "rightbutton" | "rmb" => Some(Self::Right),
            "middle" | "middleclick" | "middlebutton" | "mmb" | "wheel" | "wheelclick" => {
                Some(Self::Middle)
            }
            "back" | "backward" | "mouse4" | "xbutton1" | "x1" => Some(Self::Back),
            "forward" | "mouse5" | "xbutton2" | "x2" => Some(Self::Forward),
            _ => None,
        }
    }
}

/// Declares [`Key`]: its variants, the name each one prints as, and the extra spellings that
/// parse to it. Keeping them in one table means a key cannot be added without a name.
macro_rules! keys {
    ($($variant:ident => $label:literal $(| $alias:literal)*),+ $(,)?) => {
        /// A keyboard key a binding can press. Keys are named after their position on a US
        /// layout, like scan codes: `Z` is the key at that place whatever the layout.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Key {
            $(
                #[doc = concat!("The `", stringify!($variant), "` key.")]
                $variant,
            )+
        }

        impl Key {
            /// Every key, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// How many keys there are.
            pub const COUNT: usize = Self::ALL.len();

            /// Name used when the binding is printed.
            #[must_use]
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label,)+
                }
            }

            fn from_name(name: &str) -> Option<Self> {
                let wanted = normalize(name);
                $(
                    if wanted == normalize($label) $(|| wanted == normalize($alias))* {
                        return Some(Self::$variant);
                    }
                )+
                None
            }
        }
    };
}

keys! {
    A => "A", B => "B", C => "C", D => "D", E => "E", F => "F", G => "G", H => "H", I => "I",
    J => "J", K => "K", L => "L", M => "M", N => "N", O => "O", P => "P", Q => "Q", R => "R",
    S => "S", T => "T", U => "U", V => "V", W => "W", X => "X", Y => "Y", Z => "Z",
    Num0 => "0", Num1 => "1", Num2 => "2", Num3 => "3", Num4 => "4",
    Num5 => "5", Num6 => "6", Num7 => "7", Num8 => "8", Num9 => "9",
    F1 => "F1", F2 => "F2", F3 => "F3", F4 => "F4", F5 => "F5", F6 => "F6",
    F7 => "F7", F8 => "F8", F9 => "F9", F10 => "F10", F11 => "F11", F12 => "F12",
    F13 => "F13", F14 => "F14", F15 => "F15", F16 => "F16", F17 => "F17", F18 => "F18",
    F19 => "F19", F20 => "F20", F21 => "F21", F22 => "F22", F23 => "F23", F24 => "F24",
    Escape => "Esc" | "Escape",
    Tab => "Tab",
    Enter => "Enter" | "Return",
    Space => "Space" | "Spacebar",
    Backspace => "Backspace",
    Delete => "Delete" | "Del",
    Insert => "Insert" | "Ins",
    Home => "Home",
    End => "End",
    PageUp => "PageUp" | "PgUp" | "Prior",
    PageDown => "PageDown" | "PgDn" | "Next",
    Up => "Up" | "UpArrow",
    Down => "Down" | "DownArrow",
    Left => "Left" | "LeftArrow",
    Right => "Right" | "RightArrow",
    Ctrl => "Ctrl" | "Control" | "LeftControl" | "LControl" | "LeftCtrl",
    Shift => "Shift" | "LeftShift" | "LShift",
    Alt => "Alt" | "LeftAlt" | "LAlt",
    Meta => "Win" | "Windows" | "Meta" | "Super" | "Cmd" | "Command" | "LeftWindows",
    Minus => "-" | "Minus" | "Dash",
    Equals => "=" | "Equals" | "Equal",
    LeftBracket => "[" | "LeftBracket" | "LBracket",
    RightBracket => "]" | "RightBracket" | "RBracket",
    Backslash => "\\" | "Backslash",
    Semicolon => ";" | "Semicolon",
    Quote => "'" | "Quote" | "Apostrophe",
    Backtick => "`" | "Backtick" | "Grave" | "Tilde",
    Comma => "," | "Comma",
    Period => "." | "Period" | "Dot",
    Slash => "/" | "Slash",
}

impl Key {
    /// Whether this key is a modifier (`Ctrl`, `Shift`, `Alt`, `Win`).
    #[must_use]
    pub const fn is_modifier(self) -> bool {
        matches!(self, Self::Ctrl | Self::Shift | Self::Alt | Self::Meta)
    }
}

/// Lowercases a name and drops spaces, dashes and underscores, so `Left Click`, `left-click`
/// and `LEFTCLICK` are the same word. A lone `-` is kept, because that is the minus key.
fn normalize(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed == "-" {
        return trimmed.to_string();
    }
    trimmed
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// What a pen button or a tablet key does when it is pressed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Binding {
    /// Nothing happens.
    #[default]
    None,
    /// The mouse button is held while the physical button is held.
    Mouse(MouseButton),
    /// The keys are held, in order, while the physical button is held.
    Chord(Vec<Key>),
}

/// Why a binding text could not be understood.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum BindingError {
    /// A name in the text is neither a mouse button nor a key.
    #[error("Unknown key or button: '{0}'")]
    Unknown(String),
    /// The chord has an empty part, such as `Ctrl+`.
    #[error("A key is missing in '{0}'")]
    EmptyKey(String),
    /// The chord has more than [`MAX_CHORD_LEN`] keys.
    #[error("A binding can use at most {MAX_CHORD_LEN} keys")]
    TooLong,
    /// The same key appears twice in the chord.
    #[error("'{0}' is used twice in the binding")]
    Repeated(String),
    /// A mouse button was mixed with keys, such as `Ctrl+Left Click`.
    #[error("A mouse button cannot be combined with keys")]
    MouseInChord,
}

impl FromStr for Binding {
    type Err = BindingError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = strip_legacy_wrapper(text.trim());
        let text = text.as_ref();
        if text.is_empty() {
            return Ok(Self::None);
        }
        let flat = normalize(text);
        if matches!(flat.as_str(), "none" | "disabled" | "off" | "nothing") {
            return Ok(Self::None);
        }
        // A mouse button has a name of its own, so try the whole text first: `Left Click`
        // must not be read as the `Left` arrow key.
        if let Some(button) = MouseButton::from_name(text) {
            return Ok(Self::Mouse(button));
        }

        let mut keys: Vec<Key> = Vec::new();
        for part in text.split('+') {
            let part = part.trim();
            if part.is_empty() {
                return Err(BindingError::EmptyKey(text.to_string()));
            }
            if MouseButton::from_name(part).is_some() && !is_key_name(part) {
                return Err(BindingError::MouseInChord);
            }
            let key =
                Key::from_name(part).ok_or_else(|| BindingError::Unknown(part.to_string()))?;
            if keys.contains(&key) {
                return Err(BindingError::Repeated(key.label().to_string()));
            }
            keys.push(key);
            if keys.len() > MAX_CHORD_LEN {
                return Err(BindingError::TooLong);
            }
        }
        Ok(Self::Chord(keys))
    }
}

fn is_key_name(name: &str) -> bool {
    Key::from_name(name).is_some()
}

/// Reads the older `OpenTabletDriver` spelling `Mouse Button Binding: (Button: Left)` and
/// `Key Binding: (Key: Space)` by returning just the name inside the parentheses, written the
/// way this module spells it (`Left` becomes `Left Click`).
fn strip_legacy_wrapper(text: &str) -> Cow<'_, str> {
    let lower = text.to_ascii_lowercase();
    let is_mouse = lower.starts_with("mouse button binding");
    if !is_mouse && !lower.starts_with("key binding") {
        return Cow::Borrowed(text);
    }
    let name = text
        .split_once('(')
        .map(|(_, rest)| rest.trim_end().trim_end_matches(')'))
        .and_then(|inner| inner.split_once(':'))
        .map_or("", |(_, name)| name.trim());
    if is_mouse && matches!(normalize(name).as_str(), "left" | "right") {
        return Cow::Owned(format!("{name} Click"));
    }
    Cow::Borrowed(name)
}

impl fmt::Display for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Mouse(button) => f.write_str(button.label()),
            Self::Chord(keys) => {
                for (index, key) in keys.iter().enumerate() {
                    if index > 0 {
                        f.write_str("+")?;
                    }
                    f.write_str(key.label())?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Binding {
        text.parse().unwrap()
    }

    #[test]
    fn none_and_disabled_and_empty_mean_nothing() {
        for text in ["None", "none", "Disabled", "  ", "", "Off"] {
            assert_eq!(parse(text), Binding::None, "{text:?}");
        }
    }

    #[test]
    fn every_mouse_button_is_understood() {
        assert_eq!(parse("Left Click"), Binding::Mouse(MouseButton::Left));
        assert_eq!(parse("right click"), Binding::Mouse(MouseButton::Right));
        assert_eq!(parse("Middle Click"), Binding::Mouse(MouseButton::Middle));
        assert_eq!(parse("Back"), Binding::Mouse(MouseButton::Back));
        assert_eq!(parse("Forward"), Binding::Mouse(MouseButton::Forward));
    }

    #[test]
    fn left_click_is_a_mouse_button_but_left_alone_is_the_arrow_key() {
        assert_eq!(parse("Left Click"), Binding::Mouse(MouseButton::Left));
        assert_eq!(parse("Left"), Binding::Chord(vec![Key::Left]));
        assert_eq!(
            parse("Ctrl+Left"),
            Binding::Chord(vec![Key::Ctrl, Key::Left])
        );
    }

    #[test]
    fn a_single_key_and_a_chord_are_understood_in_order() {
        assert_eq!(parse("Space"), Binding::Chord(vec![Key::Space]));
        assert_eq!(
            parse("ctrl + shift + z"),
            Binding::Chord(vec![Key::Ctrl, Key::Shift, Key::Z])
        );
    }

    #[test]
    fn aliases_and_symbols_name_the_same_key() {
        assert_eq!(parse("Control+Del"), parse("Ctrl+Delete"));
        assert_eq!(parse("Return"), parse("Enter"));
        assert_eq!(parse("Ctrl+-"), Binding::Chord(vec![Key::Ctrl, Key::Minus]));
        assert_eq!(
            parse("Ctrl+="),
            Binding::Chord(vec![Key::Ctrl, Key::Equals])
        );
        assert_eq!(parse("["), Binding::Chord(vec![Key::LeftBracket]));
    }

    #[test]
    fn the_open_tablet_driver_spellings_still_load() {
        assert_eq!(
            parse("Mouse Button Binding: (Button: Left)"),
            Binding::Mouse(MouseButton::Left)
        );
        assert_eq!(
            parse("Mouse Button Binding: (Button: Backward)"),
            Binding::Mouse(MouseButton::Back)
        );
        assert_eq!(
            parse("Key Binding: (Key: Space)"),
            Binding::Chord(vec![Key::Space])
        );
        assert_eq!(parse("Mouse Button Binding: ()"), Binding::None);
    }

    #[test]
    fn a_binding_prints_in_the_plain_format_and_reads_back_the_same() {
        let samples = [
            Binding::None,
            Binding::Mouse(MouseButton::Right),
            Binding::Mouse(MouseButton::Back),
            Binding::Chord(vec![Key::Ctrl, Key::Shift, Key::S]),
            Binding::Chord(vec![Key::F13]),
            Binding::Chord(vec![Key::Ctrl, Key::Minus]),
        ];
        for binding in samples {
            assert_eq!(parse(&binding.to_string()), binding, "{binding}");
        }
    }

    #[test]
    fn every_key_reads_back_from_its_own_label() {
        for &key in Key::ALL {
            assert_eq!(Key::from_name(key.label()), Some(key), "{}", key.label());
        }
    }

    #[test]
    fn no_two_keys_share_a_name() {
        for (index, &key) in Key::ALL.iter().enumerate() {
            for &other in Key::ALL.iter().skip(index + 1) {
                assert_ne!(
                    normalize(key.label()),
                    normalize(other.label()),
                    "{key:?} and {other:?}"
                );
            }
        }
    }

    #[test]
    fn an_unknown_name_is_reported_with_that_name() {
        assert_eq!(
            "Ctrl+Banana".parse::<Binding>(),
            Err(BindingError::Unknown("Banana".to_string()))
        );
    }

    #[test]
    fn a_chord_with_an_empty_part_is_refused() {
        assert!(matches!(
            "Ctrl+".parse::<Binding>(),
            Err(BindingError::EmptyKey(_))
        ));
        assert!(matches!(
            "+A".parse::<Binding>(),
            Err(BindingError::EmptyKey(_))
        ));
    }

    #[test]
    fn a_chord_longer_than_the_limit_is_refused() {
        assert_eq!(
            "Ctrl+Shift+Alt+Win+A".parse::<Binding>(),
            Err(BindingError::TooLong)
        );
        assert!("Ctrl+Shift+Alt+A".parse::<Binding>().is_ok());
    }

    #[test]
    fn a_repeated_key_is_refused() {
        assert!(matches!(
            "Ctrl+A+Ctrl".parse::<Binding>(),
            Err(BindingError::Repeated(_))
        ));
    }

    #[test]
    fn a_mouse_button_cannot_join_a_chord() {
        assert_eq!(
            "Ctrl+Right Click".parse::<Binding>(),
            Err(BindingError::MouseInChord)
        );
    }

    #[test]
    fn the_mouse_button_count_matches_the_buttons() {
        let all = [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
        ];
        assert_eq!(all.len(), MouseButton::COUNT);
        for (position, button) in all.into_iter().enumerate() {
            assert_eq!(button.index(), position);
        }
    }

    #[test]
    fn keys_can_be_used_as_table_positions() {
        for (position, &key) in Key::ALL.iter().enumerate() {
            assert_eq!(key as usize, position);
        }
        assert_eq!(Key::ALL.len(), Key::COUNT);
    }

    #[test]
    fn modifiers_are_told_apart_from_other_keys() {
        assert!(Key::Ctrl.is_modifier());
        assert!(Key::Meta.is_modifier());
        assert!(!Key::A.is_modifier());
    }
}
