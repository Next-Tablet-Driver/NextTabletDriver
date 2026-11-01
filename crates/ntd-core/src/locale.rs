//! The languages of the interface.

use serde::{Deserialize, Serialize};

/// Supported application locales.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Locale {
    /// English, the default locale.
    #[default]
    English,
    /// French.
    French,
}

impl Locale {
    /// Returns the JSON filename for this locale.
    #[must_use]
    pub const fn filename(self) -> &'static str {
        match self {
            Self::English => "en.json",
            Self::French => "fr.json",
        }
    }

    /// Returns the native display name for this locale.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Français",
        }
    }

    /// Returns all available locales.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::English, Self::French]
    }
}
