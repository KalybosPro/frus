//! `frus-l10n` — **localisation** for frus applications, built on
//! [Fluent](https://projectfluent.org/), Mozilla's i18n standard.
//!
//! Nothing is reinvented here: messages, plurals and selections live in `.ftl`
//! resources, embedded by the application through `include_str!`, and resolved per
//! locale with a **fallback** chain (`fluent-langneg` negotiation).
//!
//! ```
//! use frus_l10n::{Localizer, args};
//! let mut l10n = Localizer::new("en");
//! l10n.add("en", "hello = Hello, { $name }!\ntasks = { $n ->\n    [one] { $n } task\n   *[other] { $n } tasks\n }");
//! l10n.add("fr", "hello = Bonjour, { $name } !\ntasks = { $n ->\n    [one] { $n } tâche\n   *[other] { $n } tâches\n }");
//!
//! assert_eq!(l10n.format("hello", args![name: "Ada"]), "Hello, Ada!");
//! l10n.set_locale("fr");
//! assert_eq!(l10n.format("tasks", args![n: 2]), "2 tâches");
//! // Falls back to the default locale when the key is missing from the current one.
//! ```

use std::collections::HashMap;

// The **concurrent** bundle — its memoiser sits behind a `Mutex` — is what makes
// `Localizer` `Send + Sync`, so an application can hold it in a `static`/`OnceLock`.
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// An argument value passed to a message: a string or a number.
#[derive(Clone, Debug)]
pub enum Arg<'a> {
    /// Text.
    Str(&'a str),
    /// A number, which takes part in the CLDR plural rules.
    Num(f64),
}

impl<'a> From<&'a str> for Arg<'a> {
    fn from(s: &'a str) -> Self {
        Arg::Str(s)
    }
}
impl From<i64> for Arg<'_> {
    fn from(n: i64) -> Self {
        Arg::Num(n as f64)
    }
}
impl From<i32> for Arg<'_> {
    fn from(n: i32) -> Self {
        Arg::Num(n as f64)
    }
}
impl From<usize> for Arg<'_> {
    fn from(n: usize) -> Self {
        Arg::Num(n as f64)
    }
}
impl From<f64> for Arg<'_> {
    fn from(n: f64) -> Self {
        Arg::Num(n)
    }
}

/// Builds a message's argument list: `args![name: "Ada", n: 3]`.
#[macro_export]
macro_rules! args {
    ($($key:ident : $value:expr),* $(,)?) => {
        &[$((stringify!($key), $crate::Arg::from($value))),*]
    };
    () => { &[] };
}

/// The **localiser**: Fluent bundles per locale, a current locale and a default
/// locale, the latter being the last resort.
pub struct Localizer {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
    default: LanguageIdentifier,
    current: LanguageIdentifier,
}

impl Localizer {
    /// Creates a localiser whose default — and current — locale is `default`, for
    /// instance `"en"` or `"fr-FR"`. A malformed locale falls back to `und`.
    pub fn new(default: &str) -> Self {
        let default: LanguageIdentifier = default.parse().unwrap_or_default();
        Self {
            bundles: HashMap::new(),
            current: default.clone(),
            default,
        }
    }

    /// Adds to, or extends, a locale's messages from an `.ftl` source. Syntax errors
    /// are reported through `debug_assert` but do not stop the rest of the file from
    /// loading.
    pub fn add(&mut self, locale: &str, ftl: &str) {
        let langid: LanguageIdentifier = match locale.parse() {
            Ok(id) => id,
            Err(_) => return,
        };
        let resource = match FluentResource::try_new(ftl.to_owned()) {
            Ok(res) => res,
            Err((res, _errors)) => {
                debug_assert!(false, "erreurs de syntaxe Fluent dans « {locale} »");
                res
            }
        };
        let bundle = self
            .bundles
            .entry(langid.clone())
            .or_insert_with(|| new_bundle(langid));
        // An existing key is not overwritten: add_resource skips duplicates and
        // reports an error, which we deliberately ignore.
        let _ = bundle.add_resource(resource);
    }

    /// The available locales — those for which messages have been loaded.
    pub fn available(&self) -> Vec<LanguageIdentifier> {
        self.bundles.keys().cloned().collect()
    }

    /// Sets the current locale. The best match among the available locales is
    /// **negotiated**: an exact match first (`fr-FR`), then by language (`fr-CA` →
    /// `fr`), and failing that the default locale. Returns the locale actually
    /// chosen.
    pub fn set_locale(&mut self, locale: &str) -> LanguageIdentifier {
        let requested: LanguageIdentifier = match locale.parse() {
            Ok(id) => id,
            Err(_) => {
                self.current = self.default.clone();
                return self.current.clone();
            }
        };
        // 1) An exact match.
        let exact = self.bundles.keys().find(|id| **id == requested).cloned();
        // 2) The same language, ignoring region and variant.
        let by_lang = || {
            self.bundles
                .keys()
                .find(|id| id.language == requested.language)
                .cloned()
        };
        self.current = exact
            .or_else(by_lang)
            .unwrap_or_else(|| self.default.clone());
        self.current.clone()
    }

    /// The current locale.
    pub fn locale(&self) -> &LanguageIdentifier {
        &self.current
    }

    /// Resolves a message **with no arguments**. The fallback chain is current
    /// locale → default → the key itself: never a panic, never a surprise empty string.
    pub fn get(&self, key: &str) -> String {
        self.format(key, &[])
    }

    /// Resolves a message **with arguments** (`args![…]`) in the **current locale**,
    /// with the same fallback chain as [`Localizer::get`].
    pub fn format(&self, key: &str, args: &[(&str, Arg)]) -> String {
        self.format_for(&self.current, key, args)
    }

    /// Resolves a message in an **explicit locale**, without changing the current
    /// one — handy for a pure `view` that receives the language as a parameter. The
    /// fallback chain is `locale` → the default locale → the raw key.
    pub fn format_for(
        &self,
        locale: &LanguageIdentifier,
        key: &str,
        args: &[(&str, Arg)],
    ) -> String {
        for langid in [locale, &self.default] {
            if let Some(bundle) = self.bundles.get(langid) {
                if let Some(text) = format_in(bundle, key, args) {
                    return text;
                }
            }
        }
        // Last resort: the raw key, which makes the gap visible without breaking the UI.
        key.to_owned()
    }

    /// Parses a locale tag (`"fr"`, `"fr-FR"`), or returns the default locale when
    /// it is malformed — a utility for callers of [`Localizer::format_for`].
    pub fn langid(&self, locale: &str) -> LanguageIdentifier {
        locale.parse().unwrap_or_else(|_| self.default.clone())
    }
}

/// Builds a bundle for a locale, without **bidi isolation** — no FSI/PDI marks
/// around arguments — which keeps the output readable and testable. RTL direction
/// is handled at the layout level; see `Theme::direction`.
fn new_bundle(langid: LanguageIdentifier) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    bundle.set_use_isolating(false);
    bundle
}

/// Formats `key` in `bundle` when the message exists, `None` otherwise.
fn format_in(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    args: &[(&str, Arg)],
) -> Option<String> {
    let message = bundle.get_message(key)?;
    let pattern = message.value()?;
    let mut fargs = FluentArgs::new();
    for (name, value) in args {
        let v = match value {
            Arg::Str(s) => FluentValue::from(*s),
            Arg::Num(n) => FluentValue::from(*n),
        };
        fargs.set(*name, v);
    }
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, Some(&fargs), &mut errors);
    Some(text.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Localizer {
        let mut l = Localizer::new("en");
        l.add(
            "en",
            "hello = Hello, { $name }!\n\
             tasks = { $n ->\n    [one] { $n } task\n   *[other] { $n } tasks\n }\n\
             only-en = English only",
        );
        l.add(
            "fr",
            "hello = Bonjour, { $name } !\n\
             tasks = { $n ->\n    [one] { $n } tâche\n   *[other] { $n } tâches\n }",
        );
        l
    }

    #[test]
    fn resolves_arguments() {
        let l = sample();
        assert_eq!(l.format("hello", args![name: "Ada"]), "Hello, Ada!");
    }

    #[test]
    fn plural_rules_per_locale() {
        let mut l = sample();
        assert_eq!(l.format("tasks", args![n: 1]), "1 task");
        assert_eq!(l.format("tasks", args![n: 5]), "5 tasks");
        l.set_locale("fr");
        assert_eq!(l.format("tasks", args![n: 1]), "1 tâche");
        assert_eq!(l.format("tasks", args![n: 3]), "3 tâches");
    }

    #[test]
    fn negotiates_region_to_base_language() {
        let mut l = sample();
        // fr-CA does not exist, so it is negotiated down to fr.
        let got = l.set_locale("fr-CA");
        assert_eq!(got.to_string(), "fr");
        assert_eq!(l.format("hello", args![name: "Zoé"]), "Bonjour, Zoé !");
    }

    #[test]
    fn falls_back_to_default_then_key() {
        let mut l = sample();
        l.set_locale("fr");
        // "only-en" exists only in English, so it falls back to the default locale.
        assert_eq!(l.get("only-en"), "English only");
        // A key that exists nowhere yields the raw key.
        assert_eq!(l.get("missing-key"), "missing-key");
    }

    #[test]
    fn unknown_locale_stays_on_default() {
        let mut l = sample();
        let got = l.set_locale("xx");
        assert_eq!(got.to_string(), "en");
    }
}
