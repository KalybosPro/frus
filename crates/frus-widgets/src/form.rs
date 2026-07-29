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

use frus_core::{Color, Insets, Point, Rect, Role, Scene, Semantics};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

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
/// Construit par chaînage de [`Form::field`] ; se consulte ensuite. Les **valeurs** sont
/// mémorisées pour permettre la **validation croisée** (un champ comparé à un autre).
#[derive(Default)]
pub struct Form {
    fields: Vec<(&'static str, String, Option<String>)>,
}

impl Form {
    /// Un formulaire vide (tout est valide tant qu'aucun champ n'est déclaré).
    pub fn new() -> Self {
        Self::default()
    }

    /// Valide `value` avec `rule` et enregistre le résultat sous `key`.
    pub fn field(mut self, key: &'static str, value: &str, rule: Rule) -> Self {
        let error = rule.check(value);
        self.fields.push((key, value.to_string(), error));
        self
    }

    /// Valide un champ par une **fonction croisée** qui reçoit sa valeur **et** le formulaire
    /// partiel (champs **déjà déclarés**) : `check(value, form)` peut consulter
    /// [`form.value(other)`](Self::value) — mot de passe confirmé, dates cohérentes, etc.
    /// Déclarez donc le champ référencé **avant**.
    pub fn field_with(
        mut self,
        key: &'static str,
        value: &str,
        check: impl Fn(&str, &Form) -> Option<String>,
    ) -> Self {
        let error = check(value, &self);
        self.fields.push((key, value.to_string(), error));
        self
    }

    /// Convenance de validation croisée : `value` doit **égaler** la valeur du champ `other`
    /// (déjà déclaré) — le cas « confirmer le mot de passe ». Sinon `message`.
    pub fn matches(
        self,
        key: &'static str,
        value: &str,
        other: &'static str,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        self.field_with(key, value, move |v, form| {
            (form.value(other) != Some(v)).then(|| message.clone())
        })
    }

    /// La valeur enregistrée du champ `key` (pour la validation croisée).
    pub fn value(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|(k, _, _)| *k == key).map(|(_, v, _)| v.as_str())
    }

    /// `true` si aucun champ n'est en erreur.
    pub fn is_valid(&self) -> bool {
        self.fields.iter().all(|(_, _, e)| e.is_none())
    }

    /// Le message d'erreur du champ `key`, s'il en a un.
    pub fn error(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _, _)| *k == key)
            .and_then(|(_, _, e)| e.as_deref())
    }

    /// Tous les messages d'erreur `(clé, message)`, dans l'ordre de déclaration — pour un
    /// **récapitulatif** en tête de formulaire (voir [`ErrorSummary`]).
    pub fn errors(&self) -> Vec<(&'static str, &str)> {
        self.fields
            .iter()
            .filter_map(|(k, _, e)| e.as_deref().map(|m| (*k, m)))
            .collect()
    }

    /// La clé du **premier** champ en erreur (ordre de déclaration) — typiquement
    /// celui à focaliser / mettre en avant à la soumission.
    pub fn first_invalid(&self) -> Option<&'static str> {
        self.fields
            .iter()
            .find(|(_, _, e)| e.is_some())
            .map(|(k, _, _)| *k)
    }
}

/// Marge intérieure du récapitulatif d'erreurs.
const SUMMARY_PAD: f32 = 12.0;

/// **Récapitulatif d'erreurs** : une carte teintée « erreur » listant les messages (un titre
/// « Please fix N error(s) » puis une puce par message), à poser en tête de formulaire après
/// une soumission invalide. Se construit depuis [`Form::errors`]. Vide si aucun message
/// (à ne pas afficher — voir [`is_empty`](Self::is_empty)).
///
/// Les puces sont **inertes** avec [`new`](Self::new) ; avec [`links`](Self::links) chacune
/// porte un message applicatif émis au clic — typiquement pour **focaliser** le champ fautif
/// (via `Command::focus`), le récapitulatif servant alors de table des matières des erreurs.
pub struct ErrorSummary<Msg> {
    empty: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> ErrorSummary<Msg> {
    /// Construit le récapitulatif à partir des messages (l'ordre est conservé). Puces **inertes**.
    pub fn new(messages: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::assemble(messages.into_iter().map(|m| (m.into(), None)).collect())
    }

    /// Récapitulatif **cliquable** : chaque `(message, msg)` devient une puce qui émet `msg` au
    /// clic (pour focaliser le champ correspondant). L'ordre est conservé.
    pub fn links(items: impl IntoIterator<Item = (impl Into<String>, Msg)>) -> Self {
        Self::assemble(items.into_iter().map(|(m, msg)| (m.into(), Some(msg))).collect())
    }

    /// Assemble titre + puces (cliquables si un message est fourni).
    fn assemble(items: Vec<(String, Option<Msg>)>) -> Self {
        let title = match items.len() {
            1 => "Please fix 1 error".to_string(),
            n => format!("Please fix {n} errors"),
        };
        let empty = items.is_empty();
        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::with_capacity(items.len() + 1);
        children.push(Box::new(Text::new(title).size(14.0)));
        for (message, msg) in items {
            children.push(Box::new(Bullet { label: format!("• {message}"), message: msg }));
        }
        Self { empty, children }
    }

    /// `true` si aucun message — l'appelant peut alors ne rien afficher.
    pub fn is_empty(&self) -> bool {
        self.empty
    }
}

/// Taille de police d'une puce du récapitulatif.
const BULLET_SIZE: f32 = 13.0;

/// Une ligne du récapitulatif (« • message »). **Cliquable** quand elle porte un message
/// (elle focalise alors le champ fautif) ; simple texte sinon.
struct Bullet<Msg> {
    label: String,
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Bullet<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.label, BULLET_SIZE);
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Length((measured.height + 4.0).ceil()),
            padding: Insets::new(2.0, 6.0, 2.0, 6.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Surbrillance discrète au survol/focus quand la puce est cliquable.
        let highlight = status.hover_progress.max(status.focus_progress);
        if self.message.is_some() && highlight > 0.0 {
            let tint = theme.error.fade(0.12 * highlight * o);
            scene.draw_rect(bounds, tint, theme.radius, 0.0, Color::TRANSPARENT);
        }
        scene.text(
            Point::new(bounds.x + 6.0, bounds.y + 2.0),
            self.label.clone(),
            BULLET_SIZE,
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }

    fn semantics(&self) -> Option<Semantics> {
        self.message
            .as_ref()
            .map(|_| Semantics::new(Role::Button).label(self.label.clone()).clickable())
    }
}

impl<Msg: Clone> Widget<Msg> for ErrorSummary<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            width: Dimension::Percent(1.0),
            padding: Insets::new(SUMMARY_PAD, SUMMARY_PAD, SUMMARY_PAD, SUMMARY_PAD),
            gap: 4.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Carte teintée « erreur » (fond doux + bord), sous les lignes de texte.
        let o = status.opacity;
        let bg = theme.surface.lerp(theme.error, 0.12);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.error.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
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

    #[test]
    fn cross_field_confirm_password() {
        // `confirm` doit égaler `password` (déclaré avant).
        let report = |pw: &str, cf: &str| {
            Form::new()
                .field("password", pw, Rule::min_len(8, "Too short"))
                .matches("confirm", cf, "password", "Passwords do not match")
        };
        assert_eq!(report("secret12", "secret12").error("confirm"), None, "identiques → OK");
        let bad = report("secret12", "secretXX");
        assert_eq!(bad.error("confirm"), Some("Passwords do not match"));
        assert_eq!(bad.first_invalid(), Some("confirm"));
        // `field_with` généralise : accès à une autre valeur via `form.value`.
        let dates = Form::new().field("start", "10", Rule::new(|_| None)).field_with(
            "end",
            "5",
            |v, form| {
                let start: i32 = form.value("start").unwrap_or("0").parse().unwrap_or(0);
                (v.parse::<i32>().unwrap_or(0) < start).then(|| "End before start".to_string())
            },
        );
        assert_eq!(dates.error("end"), Some("End before start"));
    }

    #[test]
    fn errors_lists_all_messages_in_order() {
        let report = Form::new()
            .field("email", "nope", Rule::email("Invalid email"))
            .field("name", "Ada", Rule::required("Required"))
            .field("password", "x", Rule::min_len(8, "Too short"));
        assert_eq!(
            report.errors(),
            vec![("email", "Invalid email"), ("password", "Too short")],
            "les valides sont omis, l'ordre conservé",
        );
    }

    #[test]
    fn error_summary_lists_messages() {
        use crate::{build_ui, Runtime, Size, Theme};
        use frus_core::Primitive;
        let summary = ErrorSummary::<()>::new(["Invalid email", "Too short"]);
        assert!(!summary.is_empty());
        let ui = build_ui(&summary, Size::new(300.0, 120.0), &Runtime::default(), &Theme::default());
        let painted = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(painted("Please fix 2 errors"), "titre du récapitulatif");
        assert!(painted("• Invalid email") && painted("• Too short"), "une puce par message");
        // Vide → à ne pas afficher.
        assert!(ErrorSummary::<()>::new(Vec::<String>::new()).is_empty());
    }

    #[test]
    fn error_summary_links_emit_focus_messages() {
        #[derive(Clone, Debug, PartialEq)]
        enum Msg {
            Focus(&'static str),
        }
        let summary = ErrorSummary::links([
            ("Invalid email", Msg::Focus("email")),
            ("Too short", Msg::Focus("password")),
        ]);
        assert!(!summary.is_empty());
        let kids = Widget::<Msg>::children(&summary);
        // [0] = titre (inerte) ; [1..] = puces cliquables, dans l'ordre.
        assert_eq!(kids[0].on_click(), None, "le titre n'est pas cliquable");
        assert_eq!(kids[1].on_click(), Some(Msg::Focus("email")));
        assert_eq!(kids[2].on_click(), Some(Msg::Focus("password")));
        assert!(kids[1].focusable(), "une puce cliquable est focalisable");
        // La variante inerte ne clique pas.
        let inert = ErrorSummary::<Msg>::new(["Invalid email"]);
        let inert_kids = Widget::<Msg>::children(&inert);
        assert_eq!(inert_kids[1].on_click(), None);
        assert!(!inert_kids[1].focusable(), "une puce inerte n'est pas focalisable");
    }
}
