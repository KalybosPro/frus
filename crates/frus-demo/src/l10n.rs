//! Translation: the three languages the demo offers, and the lookups the views
//! call. One place, so that no screen has to know how Fluent is wired up.

use crate::prelude::*;
use frus_l10n::Localizer;
use std::sync::OnceLock;

/// The demo's localizer: English, French and Arabic, loaded once from embedded Fluent
/// resources (`i18n/*.ftl`).
pub(crate) fn l10n() -> &'static Localizer {
    static L10N: OnceLock<Localizer> = OnceLock::new();
    L10N.get_or_init(|| {
        let mut l = Localizer::new("en");
        l.add("en", include_str!("../i18n/en.ftl"));
        l.add("fr", include_str!("../i18n/fr.ftl"));
        l.add("ar", include_str!("../i18n/ar.ftl"));
        l
    })
}

/// The languages the demo offers (menu label, locale code). The last one, Arabic, is
/// **right-to-left**: selecting it also mirrors the layout (bidi + mirroring).
pub(crate) const LANGS: [(&str, &str); 3] =
    [("English", "en"), ("Français", "fr"), ("العربية", "ar")];

/// Is the language at index `lang` written right to left?
pub(crate) fn lang_is_rtl(lang: usize) -> bool {
    LANGS[lang].1 == "ar"
}

/// Translates an argument-free key into the language at index `lang`.
pub(crate) fn tr(lang: usize, key: &str) -> String {
    let loc = l10n();
    loc.format_for(&loc.langid(LANGS[lang].1), key, args![])
}

/// Translates a key with a numeric argument `n` (CLDR plurals).
pub(crate) fn tr_n(lang: usize, key: &str, n: usize) -> String {
    let loc = l10n();
    loc.format_for(&loc.langid(LANGS[lang].1), key, args![n: n])
}
