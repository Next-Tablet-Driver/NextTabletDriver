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

/// Global I18N state, read-heavy and write-rare (only on language change).
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
    /// Creates a new `I18n` instance with the given locale, loading its translations
    /// alongside the English fallback.
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

    /// Loads a locale file from the embedded directory and deserializes it into a flat `HashMap`.
    fn load_locale(locale: Locale) -> HashMap<String, String> {
        let content = LOCALES_DIR
            .get_file(locale.filename())
            .and_then(|f| f.contents_utf8());
        Self::parse_locale(locale, content)
    }

    /// Deserializes the content of a locale file; a missing or invalid file gives no translation.
    fn parse_locale(locale: Locale, content: Option<&str>) -> HashMap<String, String> {
        let filename = locale.filename();
        content
            .and_then(|content| serde_json::from_str::<HashMap<String, String>>(content).ok())
            .map_or_else(|| {
                log::error!(target: "I18N", "Failed to load locale file: {filename}");
                HashMap::new()
            }, |m| {
                let len = m.len();
                log::info!(target: "I18N", "Loaded translations for locale {locale:?} ({len} keys)");
                m
            })
    }

    /// Looks up a translation key in the active dictionary.
    ///
    /// If not found, falls back to the English dictionary. If still not found,
    /// returns the key itself as a fallback string and logs a warning.
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

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_translation_lookup() {
        let _guard = TEST_MUTEX.lock().unwrap();
        set_locale(Locale::English);
        assert_eq!(translate("tabs.output"), "Output");
        assert_eq!(translate("tabs.filters"), "Filters");
    }

    #[test]
    fn test_fallback_logic() {
        let _guard = TEST_MUTEX.lock().unwrap();
        set_locale(Locale::French);
        assert_eq!(
            translate("this.key.does.not.exist.at.all"),
            "this.key.does.not.exist.at.all"
        );
        set_locale(Locale::English);
    }

    #[test]
    fn test_locale_switching() {
        let _guard = TEST_MUTEX.lock().unwrap();
        set_locale(Locale::French);
        assert_eq!(translate("tabs.output"), "Sortie");

        set_locale(Locale::English);
        assert_eq!(translate("tabs.output"), "Output");
    }

    #[test]
    fn test_interpolation() {
        let _guard = TEST_MUTEX.lock().unwrap();
        set_locale(Locale::English);
        let interpolated = translate_with("toast.profile_loaded", &[("name", "TestProfile")]);
        assert_eq!(interpolated, "Loaded profile: TestProfile");

        let macro_interpolated = t!("toast.profile_loaded", name = "TestProfile");
        assert_eq!(macro_interpolated, "Loaded profile: TestProfile");
    }

    #[test]
    fn every_locale_has_a_file_and_a_native_name() {
        assert_eq!(Locale::all(), &[Locale::English, Locale::French]);
        assert_eq!(Locale::English.filename(), "en.json");
        assert_eq!(Locale::French.filename(), "fr.json");
        assert_eq!(Locale::English.display_name(), "English");
        assert_eq!(Locale::French.display_name(), "Français");
    }

    #[test]
    fn a_missing_or_invalid_locale_file_gives_no_translation() {
        log::set_max_level(log::LevelFilter::Trace);
        assert!(I18n::parse_locale(Locale::French, None).is_empty());
        assert!(I18n::parse_locale(Locale::French, Some("{ not json")).is_empty());
        assert!(I18n::parse_locale(Locale::French, Some("[1, 2]")).is_empty());
        let parsed = I18n::parse_locale(Locale::French, Some(r#"{ "a": "b" }"#));
        assert_eq!(parsed.get("a").map(String::as_str), Some("b"));
    }

    #[test]
    fn the_embedded_locales_are_complete_enough_to_translate() {
        let english = I18n::load_locale(Locale::English);
        let french = I18n::load_locale(Locale::French);
        assert!(!english.is_empty());
        assert!(!french.is_empty());
        assert_eq!(I18n::new(Locale::English).locale, Locale::English);
        let fr = I18n::new(Locale::French);
        assert_eq!(fr.locale, Locale::French);
        assert_eq!(fr.get("tabs.output"), "Sortie");
    }

    #[test]
    fn a_key_missing_from_the_active_language_falls_back_to_english_then_to_the_key() {
        log::set_max_level(log::LevelFilter::Trace);
        let i18n = I18n {
            locale: Locale::French,
            translations: HashMap::new(),
            fallback: HashMap::from([("only.in.english".to_string(), "English text".to_string())]),
        };
        assert_eq!(i18n.get("only.in.english"), "English text");
        assert_eq!(i18n.get("nowhere"), "nowhere");
    }

    #[test]
    fn selecting_the_current_language_again_changes_nothing() {
        let _guard = TEST_MUTEX.lock().unwrap();
        log::set_max_level(log::LevelFilter::Trace);
        set_locale(Locale::French);
        assert_eq!(current_locale(), Locale::French);
        set_locale(Locale::French);
        assert_eq!(current_locale(), Locale::French);
        set_locale(Locale::English);
        assert_eq!(current_locale(), Locale::English);
    }

    // Poisons the process-wide translator, so this relies on every test running in its own
    // process (cargo nextest).
    #[test]
    fn a_poisoned_translator_still_answers_with_the_key_and_the_default_locale() {
        let _guard = TEST_MUTEX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = std::thread::spawn(|| {
            let _held = I18N.write().unwrap();
            panic!("poison the translator");
        })
        .join();
        assert_eq!(translate("tabs.output"), "tabs.output");
        assert_eq!(current_locale(), Locale::default());
        // Changing the language of a poisoned translator is a no-op.
        set_locale(Locale::French);
        assert_eq!(translate_with("a.{name}", &[("name", "b")]), "a.b");
    }
}
