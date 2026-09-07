//! **Which language the interface is in** (`locale.dart`, `app.dart:146`).
//!
//! Two halves that were both missing, and missing in a way that is easy not to notice.
//!
//! Milestone 449 gave the framework a [`Localizations`](crate::localizations)
//! table, so it could say *Retour* instead of *Back*. `frus-l10n` has had Fluent and
//! its negotiation since long before that, so an application could translate its own
//! messages. **Neither was ever told what language the device is set to.** The platform
//! knows, every one of them reports it, and nothing here read it — so an application
//! either hard-coded a language or asked the reader to pick one, on a device that already
//! knew the answer.
//!
//! This module is the missing wire: a [`Locale`], the reference's resolution between what
//! the reader prefers and what the application supports
//! ([`resolve`](crate::locale::resolve)), and an ambient scope so any widget can ask
//! [`of`](crate::locale::of) which one won.
//!
//! # It always answers
//!
//! [`of`](crate::locale::of) hands back `en` with nothing installed, the way
//! [`localizations::of`](crate::localizations::of) hands back English — and it carries the
//! same warning. A default that always works is what makes the feature safe to add and
//! what would hide the shell forgetting to install one. The guard is not the default; it
//! is a test that drives the shell.

use std::cell::RefCell;
use std::fmt;

/// **A language, and optionally the script and the country it is written in**
/// (`locale.dart`).
///
/// The language is required and the other two are not, which is the reference's shape and
/// the useful one: `fr` is an answer, `fr-CA` is a better one, and `zh-Hant-TW` needs all
/// three to be right.
///
/// Comparison is exact — `fr` is not `fr-CA`. Deciding that one of them will do for the
/// other is [`resolve`]'s work, and it takes the whole list to do it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Locale {
    language: String,
    script: Option<String>,
    country: Option<String>,
}

impl Locale {
    /// A language on its own: `Locale::new("fr")`.
    pub fn new(language: impl Into<String>) -> Locale {
        Locale {
            language: language.into().to_ascii_lowercase(),
            script: None,
            country: None,
        }
    }

    /// A language in a country: `Locale::with_country("fr", "CA")`.
    pub fn with_country(language: impl Into<String>, country: impl Into<String>) -> Locale {
        Locale::new(language).country(country)
    }

    /// Adds a **script** (`Hant`, `Latn`): the writing system, which for some languages is
    /// the question that decides legibility rather than idiom.
    #[must_use]
    pub fn script(mut self, script: impl Into<String>) -> Locale {
        self.script = Some(normalise_script(&script.into()));
        self
    }

    /// Adds a **country** (`CA`, `BR`).
    #[must_use]
    pub fn country(mut self, country: impl Into<String>) -> Locale {
        self.country = Some(country.into().to_ascii_uppercase());
        self
    }

    /// The language subtag, lowercase. Always present.
    pub fn language_code(&self) -> &str {
        &self.language
    }

    /// The script subtag, title case, if there is one.
    pub fn script_code(&self) -> Option<&str> {
        self.script.as_deref()
    }

    /// The country subtag, uppercase, if there is one.
    pub fn country_code(&self) -> Option<&str> {
        self.country.as_deref()
    }

    /// **Reads a language tag**: `fr`, `fr-CA`, `fr_CA`, `zh-Hant-TW`, `en-US.UTF-8`,
    /// `pt_BR@euro`.
    ///
    /// Both separators, because both arrive: a language tag uses `-` and a POSIX
    /// environment variable uses `_`. The encoding and modifier suffixes a Unix `LANG`
    /// carries are dropped — `en_US.UTF-8` is the `en-US` locale in a particular byte
    /// encoding, and the encoding is not part of which language this is.
    ///
    /// A subtag is read by **shape**, as BCP 47 does: four letters is a script, two
    /// letters or three digits is a region. That is what lets `zh-Hant` and `zh-TW` both
    /// be understood without a table of every code.
    ///
    /// Returns `None` for a tag with no language in it at all, including the `C` and
    /// `POSIX` locales, which name an absence of one.
    pub fn parse(tag: &str) -> Option<Locale> {
        let tag = tag.split(['.', '@']).next().unwrap_or("");
        let mut parts = tag.split(['-', '_']).filter(|part| !part.is_empty());
        let language = parts.next()?.to_ascii_lowercase();
        if !language.chars().all(|c| c.is_ascii_alphabetic())
            || language.len() < 2
            || language.len() > 3
            || language == "c"
            || language == "posix"
        {
            return None;
        }
        let mut locale = Locale::new(language);
        for part in parts {
            let alphabetic = part.chars().all(|c| c.is_ascii_alphabetic());
            if part.len() == 4 && alphabetic && locale.script.is_none() {
                locale = locale.script(part);
            } else if locale.country.is_none()
                && ((part.len() == 2 && alphabetic)
                    || (part.len() == 3 && part.chars().all(|c| c.is_ascii_digit())))
            {
                locale = locale.country(part);
            }
            // Anything else is a variant or an extension, which nothing here reads yet.
        }
        Some(locale)
    }
}

/// A script subtag is written `Hant`, not `HANT` or `hant`.
fn normalise_script(script: &str) -> String {
    let mut out = String::with_capacity(script.len());
    for (i, c) in script.chars().enumerate() {
        if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

impl fmt::Display for Locale {
    /// The **language tag**, hyphenated: `fr`, `fr-CA`, `zh-Hant-TW`. What
    /// [`Locale::parse`] reads back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.language)?;
        if let Some(script) = &self.script {
            write!(f, "-{script}")?;
        }
        if let Some(country) = &self.country {
            write!(f, "-{country}")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for Locale {
    type Err = ();

    fn from_str(s: &str) -> Result<Locale, ()> {
        Locale::parse(s).ok_or(())
    }
}

impl Default for Locale {
    /// `en`, which is what every string the framework says was written in before any of
    /// them could be translated.
    fn default() -> Locale {
        Locale::new("en")
    }
}

/// **Which of the application's languages the reader gets** — the reference's
/// `basicLocaleListResolution` (`app.dart:146`), which is what its app widget runs.
///
/// `preferred` is the reader's list, best first, as the platform reports it. `supported`
/// is what the application actually has. The answer is always one of `supported`, or its
/// first entry when nothing matches — an interface in the wrong language beats no
/// interface.
///
/// The order of attempts, per preferred locale in turn:
///
/// 1. an exact match on all three subtags;
/// 2. language and script;
/// 3. language and country;
/// 4. language alone — **held back**, not returned at once, unless it came from the
///    reader's very first choice and the next choice is a different language;
/// 5. country alone, remembered but never preferred over a language match.
///
/// The fourth rung is the one worth understanding. A language-only match is a weak match:
/// `fr` for a reader who asked for `fr-CA`. From the reader's **second** choice onward it
/// is remembered rather than returned, and only used once the round after it has failed to
/// do better — so a reader listing `[it, fr-BE, fr-CA]` gets `fr-CA` and not `fr`.
///
/// The reader's **first** choice is trusted instead: a weak match on it is taken at once,
/// because being asked for French first outranks an exact match on a language asked for
/// second. The one exception is a next choice in the same language, where waiting a round
/// cannot lose anything and may find a country match.
///
/// It does **not** consider how close two languages are to each other. Neither does the
/// reference's, which says so in the same words: German resolves to Chinese over French
/// if Chinese is listed first.
pub fn resolve(preferred: &[Locale], supported: &[Locale]) -> Locale {
    let Some(first_supported) = supported.first() else {
        return Locale::default();
    };
    // A platform that reports no locales, or one asked before it has had a chance to say:
    // the application's own first choice is the honest answer.
    if preferred.is_empty() {
        return first_supported.clone();
    }

    // The supported list is indexed once rather than searched per preferred locale, and
    // the **first** entry to claim a key keeps it: an application listing `en-US` before
    // `en-GB` has said which English it would rather give a reader who only asked for
    // `en`.
    let find = |f: &dyn Fn(&Locale) -> bool| supported.iter().find(|l| f(l)).cloned();

    let mut by_language: Option<Locale> = None;
    let mut by_country: Option<Locale> = None;

    for (index, wanted) in preferred.iter().enumerate() {
        // 1. Everything matches.
        if supported.contains(wanted) {
            return wanted.clone();
        }
        // 2. The language and the script it is written in.
        if let Some(script) = wanted.script_code() {
            if let Some(found) = find(&|l| {
                l.language_code() == wanted.language_code() && l.script_code() == Some(script)
            }) {
                return found;
            }
        }
        // 3. The language and the country.
        if let Some(country) = wanted.country_code() {
            if let Some(found) = find(&|l| {
                l.language_code() == wanted.language_code() && l.country_code() == Some(country)
            }) {
                return found;
            }
        }
        // 4. A language-only match held back from the previous round, now that this
        //    round has failed to do better than it.
        if let Some(found) = by_language.take() {
            return found;
        }
        if let Some(found) = find(&|l| l.language_code() == wanted.language_code()) {
            let next_is_same_language = preferred
                .get(index + 1)
                .is_some_and(|next| next.language_code() == wanted.language_code());
            // The reader's first choice is strongly preferred, so a language match on it
            // is taken at once — unless the next choice is the same language, where
            // waiting one round costs nothing and may find a country match.
            if index == 0 && !next_is_same_language {
                return found;
            }
            by_language = Some(found);
        }
        // 5. The country alone. A reader is likely to know a language spoken where they
        //    are, so it beats the application's default — but never a language match.
        if by_country.is_none() {
            if let Some(country) = wanted.country_code() {
                by_country = find(&|l| l.country_code() == Some(country));
            }
        }
    }

    by_language
        .or(by_country)
        .unwrap_or_else(|| first_supported.clone())
}

thread_local! {
    /// The locale in force for this thread. `None` until something installs one.
    static AMBIENT: RefCell<Option<Locale>> = const { RefCell::new(None) };
}

/// **The locale in force**, or [`Locale::default`] (`en`) when nothing has been installed.
///
/// This is the *resolved* one — one of the application's supported locales — not the
/// reader's raw first preference. A widget asking what language to write in wants the
/// language it will actually find words in.
pub fn of() -> Locale {
    AMBIENT.with(|ambient| ambient.borrow().clone().unwrap_or_default())
}

/// Installs `locale` for this thread, from now on. The shell does this every frame.
pub fn install(locale: Locale) {
    AMBIENT.with(|ambient| *ambient.borrow_mut() = Some(locale));
}

/// Runs `f` with `locale` in force, and puts back whatever was there before — including
/// when `f` panics, so one bad frame cannot leave a stale language installed for every
/// frame after it.
pub fn scope<R>(locale: Locale, f: impl FnOnce() -> R) -> R {
    let previous = AMBIENT.with(|ambient| ambient.borrow_mut().replace(locale));
    let _restore = Restore(previous);
    f()
}

/// Puts back the previous locale when dropped, panic or not.
struct Restore(Option<Locale>);

impl Drop for Restore {
    fn drop(&mut self) {
        let previous = self.0.take();
        AMBIENT.with(|ambient| *ambient.borrow_mut() = previous);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locales(tags: &[&str]) -> Vec<Locale> {
        tags.iter().filter_map(|tag| Locale::parse(tag)).collect()
    }

    /// **A tag is read by the shape of its subtags**, which is what BCP 47 says and what
    /// makes `zh-Hant` and `zh-TW` both readable without a table of every code.
    ///
    /// Both separators, because both arrive: a language tag uses `-`, a `LANG`
    /// environment variable uses `_` and carries an encoding after a dot.
    #[test]
    fn a_language_tag_is_read_by_the_shape_of_its_subtags() {
        assert_eq!(Locale::parse("fr").unwrap().to_string(), "fr");
        assert_eq!(Locale::parse("fr-CA").unwrap().to_string(), "fr-CA");
        assert_eq!(Locale::parse("fr_ca").unwrap().to_string(), "fr-CA");
        assert_eq!(
            Locale::parse("zh-Hant-TW").unwrap().to_string(),
            "zh-Hant-TW"
        );
        assert_eq!(Locale::parse("zh-hant").unwrap().to_string(), "zh-Hant");
        assert_eq!(
            Locale::parse("en_US.UTF-8").unwrap(),
            Locale::with_country("en", "US"),
            "the encoding is not part of which language this is"
        );
        assert_eq!(
            Locale::parse("pt_BR@euro").unwrap(),
            Locale::with_country("pt", "BR")
        );
        assert_eq!(
            Locale::parse("es-419").unwrap().country_code(),
            Some("419"),
            "a three-digit region is a region"
        );

        // And the tags that name no language at all.
        assert!(Locale::parse("C").is_none());
        assert!(Locale::parse("POSIX").is_none());
        assert!(Locale::parse("").is_none());
        assert!(Locale::parse("123").is_none());
    }

    /// **Comparison is exact.** `fr` is not `fr-CA`, and deciding that one will do for the
    /// other is [`resolve`]'s work — with the whole list in hand, which is the only way to
    /// do it well.
    #[test]
    fn two_locales_are_equal_or_they_are_not() {
        assert_eq!(Locale::new("FR"), Locale::new("fr"));
        assert_ne!(Locale::new("fr"), Locale::with_country("fr", "CA"));
        assert_eq!(
            Locale::with_country("fr", "ca"),
            Locale::with_country("fr", "CA")
        );
    }

    /// **The reference's resolution, rung by rung** (`app.dart:146`).
    #[test]
    fn the_rungs_of_a_locale_resolution() {
        let supported = locales(&["en", "en-GB", "fr", "fr-CA", "zh-Hant", "es-419"]);

        assert_eq!(
            resolve(&locales(&["fr-CA"]), &supported),
            Locale::with_country("fr", "CA"),
            "an exact match"
        );
        assert_eq!(
            resolve(&locales(&["zh-Hant-TW"]), &supported),
            Locale::parse("zh-Hant").unwrap(),
            "language and script, where the country is not on offer"
        );
        assert_eq!(
            resolve(&locales(&["en-GB"]), &supported),
            Locale::with_country("en", "GB"),
            "language and country"
        );
        assert_eq!(
            resolve(&locales(&["fr-FR"]), &supported),
            Locale::new("fr"),
            "the language alone, from the reader's first choice"
        );
        assert_eq!(
            resolve(&locales(&["de"]), &supported),
            Locale::new("en"),
            "and the application's first, when nothing matches at all"
        );
        assert_eq!(
            resolve(&[], &supported),
            Locale::new("en"),
            "as when the platform reports nothing"
        );
    }

    /// **A language-only match waits one round — from the second choice onward.**
    ///
    /// It is a weak match: `fr` for a reader who asked for `fr-CA`. Held back, it lets the
    /// next preference show a better one; if none does, it stands.
    ///
    /// The reader's **first** choice is trusted rather than held back, and that is not an
    /// optimisation — it is the answer. A reader whose list begins with `fr-CA` gets
    /// French even where the application matches their *second* choice exactly, because
    /// asking for French first outranks asking for Canadian English second. The exception
    /// is a next choice in the same language, where waiting a round can only improve
    /// things.
    #[test]
    fn a_weak_language_match_waits_for_a_better_one() {
        let supported = locales(&["en", "fr", "fr-CA"]);

        // Second choice: held back, and the third does better.
        assert_eq!(
            resolve(&locales(&["it", "fr-BE", "fr-CA"]), &supported),
            Locale::with_country("fr", "CA"),
            "the round after it found the country, so the weak match gave way"
        );

        // Held back, and nothing better turned up: it stands.
        assert_eq!(
            resolve(&locales(&["it", "fr-BE", "de"]), &supported),
            Locale::new("fr")
        );

        // First choice: taken at once, even though the second choice matches exactly.
        assert_eq!(
            resolve(&locales(&["fr-CH", "en"]), &locales(&["en", "fr", "de"])),
            Locale::new("fr"),
            "asked for French first, and French is here"
        );

        // Unless the next choice is the same language, where waiting finds the country.
        assert_eq!(
            resolve(&locales(&["fr-BE", "fr-CA"]), &supported),
            Locale::with_country("fr", "CA")
        );
    }

    /// **A country beats the application's default and loses to any language match**
    /// (`app.dart:227`): a reader is likely to know a language spoken where they are, and
    /// that is still a guess about them rather than a statement by them.
    #[test]
    fn a_country_is_the_last_thing_tried() {
        let supported = locales(&["en", "fr-CA"]);
        assert_eq!(
            resolve(&locales(&["it-CA"]), &supported),
            Locale::with_country("fr", "CA"),
            "no Italian, but Canada is on the list"
        );
        assert_eq!(
            resolve(&locales(&["it-CA", "en-AU"]), &supported),
            Locale::new("en"),
            "and a language match outranks it, however late it arrives"
        );
    }

    /// **The application's order decides ties**: listing `en-US` before `en-GB` says which
    /// English a reader who only asked for `en` should get.
    #[test]
    fn the_application_s_order_decides_which_of_two_will_do() {
        let supported = locales(&["en-US", "en-GB"]);
        assert_eq!(
            resolve(&locales(&["en"]), &supported),
            Locale::with_country("en", "US")
        );
        let supported = locales(&["en-GB", "en-US"]);
        assert_eq!(
            resolve(&locales(&["en"]), &supported),
            Locale::with_country("en", "GB")
        );
    }

    /// **The ambient scope answers, and puts back what it found** — including through a
    /// panic, so one bad frame cannot leave the wrong language installed for every frame
    /// after it.
    #[test]
    fn the_scope_answers_and_restores() {
        assert_eq!(of(), Locale::new("en"));
        scope(Locale::with_country("fr", "CA"), || {
            assert_eq!(of(), Locale::with_country("fr", "CA"));
            scope(Locale::new("de"), || assert_eq!(of(), Locale::new("de")));
            assert_eq!(of(), Locale::with_country("fr", "CA"), "and nests");
        });
        assert_eq!(of(), Locale::new("en"));

        let caught = std::panic::catch_unwind(|| {
            scope(Locale::new("de"), || panic!("mid-frame"));
        });
        assert!(caught.is_err());
        assert_eq!(of(), Locale::new("en"), "a panic put the previous one back");
    }
}
