//! # Internationalization (I18N)
//!
//! Provides a simple, compile-time embedded locale system.
//! Locale files (JSON) are embedded into the binary via `include_dir!`.
//! The active locale can be switched at runtime without restarting.
//!
//! ## Usage
//! ```ignore
//! use next_tablet_driver::t;
//! let label = t!("tabs.output");
//! let msg = t!("toast.profile_loaded", name = "MyProfile");
//! ```

use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// Embedded locale directory, compiled into the binary.
static LOCALES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/resources/locales");

/// Global I18N state — read-heavy, write-rare (only on language change).
static I18N: LazyLock<RwLock<I18n>> = LazyLock::new(|| RwLock::new(I18n::new(Locale::default())));

/// Supported application locales.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Locale {
    #[default]
    English,
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

/// Holds the currently active translations.
struct I18n {
    /// Active locale.
    locale: Locale,
    /// Translations for the active locale.
    translations: HashMap<String, String>,
    /// English fallback translations (always loaded).
    fallback: HashMap<String, String>,
}

impl I18n {
    /// Creates a new `I18n` instance with the given locale.
    fn new(locale: Locale) -> Self {
        let fallback = Self::load_locale(Locale::English);
        let translations = if locale == Locale::English {
            fallback.clone()
        } else {
            Self::load_locale(locale)
        };
        Self {
            locale,
            translations,
            fallback,
        }
    }

    /// Loads a locale file from the embedded directory into a flat `HashMap`.
    fn load_locale(locale: Locale) -> HashMap<String, String> {
        let filename = locale.filename();
        LOCALES_DIR
            .get_file(filename)
            .and_then(|f| f.contents_utf8())
            .and_then(|content| serde_json::from_str::<HashMap<String, String>>(content).ok())
            .unwrap_or_else(|| {
                log::error!(target: "I18N", "Failed to load locale file: {filename}");
                HashMap::new()
            })
    }

    /// Looks up a translation key, falling back to English, then to the raw key.
    fn get(&self, key: &str) -> String {
        self.translations
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| {
                log::warn!(target: "I18N", "Missing translation key: {key}");
                key.to_string()
            })
    }
}

/// Changes the active locale at runtime. Thread-safe.
///
/// All subsequent calls to `t!()` will use the new locale immediately.
pub fn set_locale(locale: Locale) {
    if let Ok(mut i18n) = I18N.write() {
        if i18n.locale == locale {
            return;
        }
        *i18n = I18n::new(locale);
        log::info!(target: "I18N", "Locale changed to {locale:?}");
    }
}

/// Returns the currently active locale.
#[must_use]
pub fn current_locale() -> Locale {
    I18N.read().map(|i18n| i18n.locale).unwrap_or_default()
}

/// Looks up a translation by key. Used internally by the `t!()` macro.
#[must_use]
pub fn translate(key: &str) -> String {
    I18N.read()
        .map_or_else(|_| key.to_string(), |i18n| i18n.get(key))
}

/// Translates a key and performs string interpolation.
///
/// Replaces `{name}` placeholders with the provided values.
#[must_use]
pub fn translate_with(key: &str, args: &[(&str, &str)]) -> String {
    let mut result = translate(key);
    for (name, value) in args {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

/// Macro for looking up a translated string.
///
/// # Examples
/// ```ignore
/// t!("tabs.output")                                // Simple lookup
/// t!("toast.profile_loaded", name = "Default")     // With interpolation
/// ```
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::translate($key)
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::translate_with($key, &[
            $( (stringify!($name), &format!("{}", $value)) ),+
        ])
    };
}
