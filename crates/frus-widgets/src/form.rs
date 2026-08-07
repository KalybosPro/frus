//! Form validation — **pure**, application-side, in the Elm spirit.
//!
//! Two bricks that combine:
//!
//! - [`Rule`]: one field's validation rule (`&str -> Option<String>`,
//!   `Some(message)` when invalid), with ready-made constructors (`required`,
//!   `min_len`, `email`…) and a [`Rule::all`] combinator (the first failing rule
//!   wins).
//! - [`Form`]: validates a **set** of fields declared in order, then answers three
//!   questions — is everything valid? what is field `key`'s error? which is the
//!   **first** failing field, the one to focus?
//!
//! Nothing here draws: the application calls [`Form::error`] to feed a
//! [`crate::TextInput`]'s `error(...)`, and [`Form::first_invalid`] to target the
//! field to bring out. Validity stays a **pure function of the state**.
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

/// One field's validation rule: returns `Some(message)` if the value is invalid,
/// `None` if it passes.
pub struct Rule(Box<dyn Fn(&str) -> Option<String>>);

impl Rule {
    /// An arbitrary rule from a closure.
    pub fn new(check: impl Fn(&str) -> Option<String> + 'static) -> Self {
        Self(Box::new(check))
    }

    /// Applies the rule to `value`.
    pub fn check(&self, value: &str) -> Option<String> {
        (self.0)(value)
    }

    /// Non-empty (leading and trailing whitespace ignored).
    pub fn required(message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (v.trim().is_empty()).then(|| message.clone()))
    }

    /// At least `n` characters (counted as `char`s, whitespace included).
    pub fn min_len(n: usize, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (v.chars().count() < n).then(|| message.clone()))
    }

    /// At most `n` characters.
    pub fn max_len(n: usize, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (v.chars().count() > n).then(|| message.clone()))
    }

    /// A plausible e-mail shape: `local@domain`, a non-empty local part, and a domain
    /// containing a dot that is not flush against the edges (a heuristic, not the RFC).
    pub fn email(message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(move |v| (!is_email(v)).then(|| message.clone()))
    }

    /// Combines rules: the **first** to fail, in order, wins.
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
    // A single at-sign, and a non-empty local part.
    if local.is_empty() || domain.contains('@') {
        return false;
    }
    // The domain: at least one dot, and no empty label (no `.x`, no `x.`, no `a..b`).
    domain.contains('.') && domain.split('.').all(|label| !label.is_empty())
}

/// The validation report of a set of fields, in declaration order. Built by chaining
/// [`Form::field`], then consulted. The **values** are recorded so that **cross-field
/// validation** is possible: one field compared with another.
#[derive(Default)]
pub struct Form {
    fields: Vec<(&'static str, String, Option<String>)>,
}

impl Form {
    /// An empty form (everything is valid while no field is declared).
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates `value` with `rule` and records the result under `key`.
    pub fn field(mut self, key: &'static str, value: &str, rule: Rule) -> Self {
        let error = rule.check(value);
        self.fields.push((key, value.to_string(), error));
        self
    }

    /// Validates a field with a **cross-field** function receiving its value **and** the
    /// partial form (the fields **already declared**): `check(value, form)` can consult
    /// [`form.value(other)`](Self::value) — a confirmed password, consistent dates, and
    /// so on. So declare the referenced field **first**.
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

    /// A cross-field convenience: `value` must **equal** field `other`'s value (already
    /// declared) — the "confirm the password" case. Otherwise `message`.
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

    /// Field `key`'s recorded value, for cross-field validation.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _, _)| *k == key)
            .map(|(_, v, _)| v.as_str())
    }

    /// `true` if no field is in error.
    pub fn is_valid(&self) -> bool {
        self.fields.iter().all(|(_, _, e)| e.is_none())
    }

    /// Field `key`'s error message, if it has one.
    pub fn error(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _, _)| *k == key)
            .and_then(|(_, _, e)| e.as_deref())
    }

    /// Every error message as `(key, message)`, in declaration order — for a **summary**
    /// at the head of the form (see [`ErrorSummary`]).
    pub fn errors(&self) -> Vec<(&'static str, &str)> {
        self.fields
            .iter()
            .filter_map(|(k, _, e)| e.as_deref().map(|m| (*k, m)))
            .collect()
    }

    /// The key of the **first** field in error (declaration order) — typically the
    /// one to focus or bring out on submission.
    pub fn first_invalid(&self) -> Option<&'static str> {
        self.fields
            .iter()
            .find(|(_, _, e)| e.is_some())
            .map(|(k, _, _)| *k)
    }
}

/// The error summary's inner padding.
const SUMMARY_PAD: f32 = 12.0;

/// An **error summary**: an "error"-tinted card listing the messages (a "Please fix N
/// error(s)" title, then one bullet per message), to place at the head of a form after
/// an invalid submission. Built from [`Form::errors`]. Empty when there is no message,
/// and then it should not be displayed — see [`is_empty`](Self::is_empty).
///
/// The bullets are **inert** with [`new`](Self::new); with [`links`](Self::links) each
/// one carries an application message emitted on click — typically to **focus** the
/// offending field (via `Command::focus`), the summary then acting as a table of
/// contents of the errors.
pub struct ErrorSummary<Msg> {
    empty: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> ErrorSummary<Msg> {
    /// Builds the summary from the messages (the order is preserved). **Inert** bullets.
    pub fn new(messages: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::assemble(messages.into_iter().map(|m| (m.into(), None)).collect())
    }

    /// A **clickable** summary: each `(message, msg)` becomes a bullet that emits `msg` on
    /// click, to focus the matching field. The order is preserved.
    pub fn links(items: impl IntoIterator<Item = (impl Into<String>, Msg)>) -> Self {
        Self::assemble(
            items
                .into_iter()
                .map(|(m, msg)| (m.into(), Some(msg)))
                .collect(),
        )
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
            children.push(Box::new(Bullet {
                label: format!("• {message}"),
                message: msg,
            }));
        }
        Self { empty, children }
    }

    /// `true` when there is no message — the caller can then display nothing.
    pub fn is_empty(&self) -> bool {
        self.empty
    }
}

/// The font size of a summary bullet.
const BULLET_SIZE: f32 = 13.0;

/// One summary line ("• message"). **Clickable** when it carries a message (it then
/// focuses the offending field); plain text otherwise.
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
        // A discreet highlight on hover or focus when the bullet is clickable.
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
        self.message.as_ref().map(|_| {
            Semantics::new(Role::Button)
                .label(self.label.clone())
                .clickable()
        })
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
        // The "error"-tinted card (a soft background + a border), under the text lines.
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
        let rule = Rule::all([Rule::required("Required"), Rule::email("Invalid email")]);
        // Empty → the first rule (required) wins.
        assert_eq!(rule.check(""), Some("Required".to_string()));
        // Non-empty but not an e-mail → the second.
        assert_eq!(rule.check("nope"), Some("Invalid email".to_string()));
        // Valid → no error.
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
        assert_eq!(report.error("name"), None, "a valid field → no error");
        assert_eq!(
            report.first_invalid(),
            Some("email"),
            "the first to fail, in order"
        );
    }

    #[test]
    fn empty_form_is_valid() {
        let report = Form::new();
        assert!(report.is_valid());
        assert_eq!(report.first_invalid(), None);
    }

    #[test]
    fn cross_field_confirm_password() {
        // `confirm` must equal `password`, which is declared before it.
        let report = |pw: &str, cf: &str| {
            Form::new()
                .field("password", pw, Rule::min_len(8, "Too short"))
                .matches("confirm", cf, "password", "Passwords do not match")
        };
        assert_eq!(
            report("secret12", "secret12").error("confirm"),
            None,
            "identiques → OK"
        );
        let bad = report("secret12", "secretXX");
        assert_eq!(bad.error("confirm"), Some("Passwords do not match"));
        assert_eq!(bad.first_invalid(), Some("confirm"));
        // `field_with` generalises: access to another value through `form.value`.
        let dates = Form::new()
            .field("start", "10", Rule::new(|_| None))
            .field_with("end", "5", |v, form| {
                let start: i32 = form.value("start").unwrap_or("0").parse().unwrap_or(0);
                (v.parse::<i32>().unwrap_or(0) < start).then(|| "End before start".to_string())
            });
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
            "the valid ones are omitted, the order preserved",
        );
    }

    #[test]
    fn error_summary_lists_messages() {
        use crate::{build_ui, Runtime, Size, Theme};
        use frus_core::Primitive;
        let summary = ErrorSummary::<()>::new(["Invalid email", "Too short"]);
        assert!(!summary.is_empty());
        let ui = build_ui(
            &summary,
            Size::new(300.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let painted = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(painted("Please fix 2 errors"), "the summary's title");
        assert!(
            painted("• Invalid email") && painted("• Too short"),
            "one bullet per message"
        );
        // Empty → not to be displayed.
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
        // [0] = the title (inert); [1..] = clickable bullets, in order.
        assert_eq!(kids[0].on_click(), None, "the title is not clickable");
        assert_eq!(kids[1].on_click(), Some(Msg::Focus("email")));
        assert_eq!(kids[2].on_click(), Some(Msg::Focus("password")));
        assert!(kids[1].focusable(), "a clickable bullet is focusable");
        // The inert variant does not click.
        let inert = ErrorSummary::<Msg>::new(["Invalid email"]);
        let inert_kids = Widget::<Msg>::children(&inert);
        assert_eq!(inert_kids[1].on_click(), None);
        assert!(
            !inert_kids[1].focusable(),
            "an inert bullet is not focusable"
        );
    }
}
