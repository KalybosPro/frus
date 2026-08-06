//! `frus-l10n` — **localisation** des applications frus, bâtie sur
//! [Fluent](https://projectfluent.org/) (le standard i18n de Mozilla).
//!
//! On ne réinvente rien : les messages, pluriels et sélections vivent dans des
//! ressources `.ftl` (embarquées par l'app via `include_str!`), résolus par
//! locale avec **repli** (négociation `fluent-langneg`).
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
//! // Repli sur la locale par défaut si la clé manque dans la locale courante.
//! ```

use std::collections::HashMap;

// Le bundle **concurrent** (mémoïseur derrière un `Mutex`) rend le `Localizer`
// `Send + Sync` : utilisable dans un `static`/`OnceLock` côté application.
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// Une valeur d'argument passée à un message (chaîne ou nombre).
#[derive(Clone, Debug)]
pub enum Arg<'a> {
    /// Texte.
    Str(&'a str),
    /// Nombre (participe aux règles de pluriel CLDR).
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

/// Construit la liste d'arguments d'un message : `args![name: "Ada", n: 3]`.
#[macro_export]
macro_rules! args {
    ($($key:ident : $value:expr),* $(,)?) => {
        &[$((stringify!($key), $crate::Arg::from($value))),*]
    };
    () => { &[] };
}

/// Le **localiseur** : des bundles Fluent par locale, une locale courante et
/// une locale par défaut (le repli ultime).
pub struct Localizer {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
    default: LanguageIdentifier,
    current: LanguageIdentifier,
}

impl Localizer {
    /// Crée un localiseur dont la locale par défaut (et courante) est `default`
    /// (p. ex. `"en"`, `"fr-FR"`). Une locale mal formée retombe sur `und`.
    pub fn new(default: &str) -> Self {
        let default: LanguageIdentifier = default.parse().unwrap_or_default();
        Self {
            bundles: HashMap::new(),
            current: default.clone(),
            default,
        }
    }

    /// Ajoute (ou complète) les messages d'une locale depuis une source `.ftl`.
    /// Les erreurs de syntaxe sont journalisées via `debug_assert` mais
    /// n'empêchent pas de charger le reste du fichier.
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
        // Une clé déjà présente n'est pas écrasée (add_resource ignore les
        // doublons en signalant une erreur qu'on ignore volontairement).
        let _ = bundle.add_resource(resource);
    }

    /// Locales disponibles (celles pour lesquelles des messages ont été chargés).
    pub fn available(&self) -> Vec<LanguageIdentifier> {
        self.bundles.keys().cloned().collect()
    }

    /// Fixe la locale courante. On **négocie** la meilleure correspondance parmi
    /// les locales disponibles : correspondance exacte d'abord (`fr-FR`), puis
    /// par langue (`fr-CA` → `fr`) ; à défaut, la locale par défaut. Renvoie la
    /// locale effectivement retenue.
    pub fn set_locale(&mut self, locale: &str) -> LanguageIdentifier {
        let requested: LanguageIdentifier = match locale.parse() {
            Ok(id) => id,
            Err(_) => {
                self.current = self.default.clone();
                return self.current.clone();
            }
        };
        // 1) correspondance exacte.
        let exact = self.bundles.keys().find(|id| **id == requested).cloned();
        // 2) même langue (région/variante ignorées).
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

    /// La locale courante.
    pub fn locale(&self) -> &LanguageIdentifier {
        &self.current
    }

    /// Résout un message **sans argument**. Repli : locale courante → défaut →
    /// la clé elle-même (jamais de panique, jamais de chaîne vide inattendue).
    pub fn get(&self, key: &str) -> String {
        self.format(key, &[])
    }

    /// Résout un message **avec arguments** (`args![…]`) dans la **locale
    /// courante**. Même repli que [`Localizer::get`].
    pub fn format(&self, key: &str, args: &[(&str, Arg)]) -> String {
        self.format_for(&self.current, key, args)
    }

    /// Résout un message dans une **locale explicite** (sans changer la locale
    /// courante) — pratique pour une `view` pure qui reçoit la langue en
    /// paramètre. Repli : `locale` → locale par défaut → la clé brute.
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
        // Dernier repli : la clé brute (rend le manque visible sans casser l'UI).
        key.to_owned()
    }

    /// Parse une étiquette de locale (`"fr"`, `"fr-FR"`), ou la locale par
    /// défaut si elle est mal formée — utilitaire pour les appelants de
    /// [`Localizer::format_for`].
    pub fn langid(&self, locale: &str) -> LanguageIdentifier {
        locale.parse().unwrap_or_else(|_| self.default.clone())
    }
}

/// Construit un bundle pour une locale, sans **isolation bidi** (pas de marques
/// FSI/PDI autour des arguments) : sortie lisible et testable. La direction
/// RTL est gérée au niveau de la mise en page (voir `Theme::direction`).
fn new_bundle(langid: LanguageIdentifier) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    bundle.set_use_isolating(false);
    bundle
}

/// Formate `key` dans `bundle` si le message existe, sinon `None`.
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
        // fr-CA n'existe pas → négocié vers fr.
        let got = l.set_locale("fr-CA");
        assert_eq!(got.to_string(), "fr");
        assert_eq!(l.format("hello", args![name: "Zoé"]), "Bonjour, Zoé !");
    }

    #[test]
    fn falls_back_to_default_then_key() {
        let mut l = sample();
        l.set_locale("fr");
        // « only-en » n'existe qu'en anglais → repli sur la locale par défaut.
        assert_eq!(l.get("only-en"), "English only");
        // Clé inexistante partout → la clé brute.
        assert_eq!(l.get("missing-key"), "missing-key");
    }

    #[test]
    fn unknown_locale_stays_on_default() {
        let mut l = sample();
        let got = l.set_locale("xx");
        assert_eq!(got.to_string(), "en");
    }
}
