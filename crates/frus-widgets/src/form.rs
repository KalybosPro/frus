//! Validation de formulaire — **pure**, côté application (esprit Elm).
//!
//! Deux briques qui se combinent :
//!
//! - [`Rule`] : une règle de validation d'un champ (`&str -> Option<String>`,
//!   `Some(message)` si invalide), avec des constructeurs prêts à l'emploi
//!   (`required`, `min_len`, `email`…) et un combinateur [`Rule::all`] (la première
//!   règle en échec gagne).
//! - [`Form`] : valide un **ensemble** de champs déclarés dans l'ordre, puis répond
//!   à trois questions — tout est-il valide ? quelle est l'erreur du champ `key` ?
//!   quel est le **premier** champ en échec (à focaliser) ?
//!
//! Rien ici ne dessine : l'application appelle [`Form::error`] pour alimenter le
//! `error(...)` d'un [`crate::TextInput`], et [`Form::first_invalid`] pour cibler le
//! champ à mettre en avant. La validité reste une **fonction pure de l'état**.
//!
//! ```
//! use frus_widgets::form::{Form, Rule};
//!
//! let report = Form::new()
//!     .field("email", "ada@", Rule::all([
//!         Rule::required("Required"),
//!         Rule::email("Enter a valid email address"),
//!     ]))
//!     .field("password", "secret", Rule::min_len(8, "At least 8 characters"));
//!
//! assert!(!report.is_valid());
//! assert_eq!(report.error("email"), Some("Enter a valid email address"));
//! assert_eq!(report.first_invalid(), Some("email"));
//! ```

/// Une règle de validation d'un champ : rend `Some(message)` si la valeur est
/// invalide, `None` si elle passe.
pub struct Rule(Box<dyn Fn(&str) -> Option<String>>);

impl Rule {
    /// Règle arbitraire depuis une closure.
    pub fn new(check: impl Fn(&str) -> Option<String> + 'static) -> Self {
        Self(Box::new(check))
    }

    /// Applique la règle à `value`.
    pub fn check(&self, value: &str) -> Option<String> {
        (self.0)(value)
    }

    /// Non vide (espaces de début/fin ignorés).
    pub fn required(message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (v.trim().is_empty()).then(|| message.clone()))
    }

    /// Au moins `n` caractères (comptés en `char`, espaces compris).
    pub fn min_len(n: usize, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (v.chars().count() < n).then(|| message.clone()))
    }

    /// Au plus `n` caractères.
    pub fn max_len(n: usize, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (v.chars().count() > n).then(|| message.clone()))
    }

    /// Forme d'e-mail plausible : `local@domaine`, partie locale non vide, domaine
    /// contenant un point non collé aux bords (heuristique, pas la RFC).
    pub fn email(message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (!is_email(v)).then(|| message.clone()))
    }

    /// Combine des règles : la **première** en échec (dans l'ordre) l'emporte.
    pub fn all(rules: impl IntoIterator<Item = Rule>) -> Self {
        let rules: Vec<Rule> = rules.into_iter().collect();
        Self::new(move |v| rules.iter().find_map(|r| r.check(v)))
    }
}

/// Heuristique d'e-mail (voir [`Rule::email`]).
fn is_email(v: &str) -> bool {
    let v = v.trim();
    let Some((local, domain)) = v.split_once('@') else {
        return false;
    };
    // Une seule arobase, partie locale non vide.
    if local.is_empty() || domain.contains('@') {
        return false;
    }
    // Domaine : au moins un point, et aucune étiquette vide (ni `.x`, ni `x.`, ni `a..b`).
    domain.contains('.') && domain.split('.').all(|label| !label.is_empty())
}

/// Rapport de validation d'un ensemble de champs, dans l'ordre de déclaration.
/// Construit par chaînage de [`Form::field`] ; se consulte ensuite.
#[derive(Default)]
pub struct Form {
    fields: Vec<(&'static str, Option<String>)>,
}

impl Form {
    /// Un formulaire vide (tout est valide tant qu'aucun champ n'est déclaré).
    pub fn new() -> Self {
        Self::default()
    }

    /// Valide `value` avec `rule` et enregistre le résultat sous `key`.
    pub fn field(mut self, key: &'static str, value: &str, rule: Rule) -> Self {
        self.fields.push((key, rule.check(value)));
        self
    }

    /// `true` si aucun champ n'est en erreur.
    pub fn is_valid(&self) -> bool {
        self.fields.iter().all(|(_, e)| e.is_none())
    }

    /// Le message d'erreur du champ `key`, s'il en a un.
    pub fn error(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| *k == key)
            .and_then(|(_, e)| e.as_deref())
    }

    /// La clé du **premier** champ en erreur (ordre de déclaration) — typiquement
    /// celui à focaliser / mettre en avant.
    pub fn first_invalid(&self) -> Option<&'static str> {
        self.fields
            .iter()
            .find(|(_, e)| e.is_some())
            .map(|(k, _)| *k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_rejects_blank() {
        let rule = Rule::required("Required");
        assert_eq!(rule.check("   "), Some("Required".to_string()));
        assert_eq!(rule.check("a"), None);
    }

    #[test]
    fn length_bounds() {
        assert!(Rule::min_len(3, "short").check("ab").is_some());
        assert!(Rule::min_len(3, "short").check("abc").is_none());
        assert!(Rule::max_len(3, "long").check("abcd").is_some());
    }

    #[test]
    fn email_heuristic() {
        let rule = Rule::email("bad");
        assert!(rule.check("ada@example.com").is_none());
        assert!(rule.check("ada@mail.example.co").is_none());
        assert!(rule.check("ada@").is_some());
        assert!(rule.check("ada").is_some());
        assert!(rule.check("@example.com").is_some());
        assert!(rule.check("ada@example").is_some());
    }

    #[test]
    fn all_returns_the_first_failure() {
        let rule = Rule::all([
            Rule::required("Required"),
            Rule::email("Invalid email"),
        ]);
        // Vide → la première règle (required) l'emporte.
        assert_eq!(rule.check(""), Some("Required".to_string()));
        // Non vide mais pas un e-mail → la seconde.
        assert_eq!(rule.check("nope"), Some("Invalid email".to_string()));
        // Valide → aucune erreur.
        assert_eq!(rule.check("ada@example.com"), None);
    }

    #[test]
    fn form_reports_validity_errors_and_first_invalid() {
        let report = Form::new()
            .field("email", "nope", Rule::email("Invalid email"))
            .field("password", "short", Rule::min_len(8, "Too short"))
            .field("name", "Ada", Rule::required("Required"));
        assert!(!report.is_valid());
        assert_eq!(report.error("email"), Some("Invalid email"));
        assert_eq!(report.error("password"), Some("Too short"));
        assert_eq!(report.error("name"), None, "champ valide → pas d'erreur");
        assert_eq!(report.first_invalid(), Some("email"), "premier en échec, dans l'ordre");
    }

    #[test]
    fn empty_form_is_valid() {
        let report = Form::new();
        assert!(report.is_valid());
        assert_eq!(report.first_invalid(), None);
    }
}
