//! [`SnackBar`]: a transient notification (a styled card). The *system* around it
//! (stacking, the auto-dismiss timer) is the application's responsibility,
//! typically through a timed `Command`.

use std::collections::VecDeque;

use frus_core::{
    Color, Insets, Point, Rect, ResolvedTextStyle, Role, Scene, SemanticsProperties, TextStyle,
};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 12.0;
const ACCENT: f32 = 4.0;
/// The action button (Material's "UNDO"): padding and height.
const ACTION_PAD_X: f32 = 12.0;
const ACTION_GAP: f32 = 8.0;
const ACTION_H: f32 = 32.0;

/// The message's style: what the caller said, else what the theme says, else the
/// reference's — a snackbar's content is `bodyMedium`.
///
/// **Resolved once**, so that the number the box is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403); a size that never passes through it is a size the reader cannot change.
fn content_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.snack_bar.content_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_medium)
        .resolved()
}

/// The action's style — the reference's is `labelLarge`. See [`content_style`].
fn action_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.snack_bar.action_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_large)
        .resolved()
}

/// The width an action label needs, its own padding included.
fn action_width(label: &str, style: &ResolvedTextStyle) -> f32 {
    (frus_text::measure_resolved(label, style).width + ACTION_PAD_X * 2.0).ceil()
}

/// The nature of a notification (its accent color).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SnackBarKind {
    Info,
    Success,
    Error,
}

/// A transient notification, with an optional **action** (Material Snackbar style:
/// "UNDO"). The action is a text button on the right that emits a message on click.
pub struct SnackBar<Msg> {
    text: String,
    kind: SnackBarKind,
    content_text_style: Option<TextStyle>,
    action_text_style: Option<TextStyle>,
    /// The action's label and message, kept **beside** the child rather than only inside
    /// it: the width it reserves is a measurement, and a measurement cannot be taken in a
    /// builder, before any theme exists to say what type it is in.
    action: Option<(String, Msg)>,
    /// Empty, or `[action button]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SnackBar<Msg> {
    /// Creates an informational notification.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SnackBarKind::Info,
            content_text_style: None,
            action_text_style: None,
            action: None,
            children: Vec::new(),
        }
    }

    /// The message's type, over the theme's and the reference's.
    #[must_use]
    pub fn content_text_style(mut self, style: TextStyle) -> Self {
        self.content_text_style = Some(style);
        self
    }

    /// The action's type, over the theme's and the reference's.
    #[must_use]
    pub fn action_text_style(mut self, style: TextStyle) -> Self {
        self.action_text_style = Some(style);
        self.rebuild_action();
        self
    }

    /// The success variant.
    pub fn success(mut self) -> Self {
        self.kind = SnackBarKind::Success;
        self
    }

    /// Variante erreur.
    pub fn error(mut self) -> Self {
        self.kind = SnackBarKind::Error;
        self
    }

    /// Adds an **action button** (an uppercased label, Material style) that emits `message`
    /// on click — typically "UNDO", to undo whatever triggered the notification.
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.action = Some((label.into().to_uppercase(), message));
        self.rebuild_action();
        self
    }

    /// Carries the current action *and the current style* into the child.
    ///
    /// Called by both builders so the two are order-independent: `.action(…)` then
    /// `.action_text_style(…)` and the reverse describe the same notification, which a
    /// caller is entitled to assume and would otherwise have to discover.
    fn rebuild_action(&mut self) {
        self.children = match &self.action {
            Some((label, message)) => vec![Box::new(ActionButton {
                label: label.clone(),
                text_style: self.action_text_style,
                message: message.clone(),
            })],
            None => Vec::new(),
        };
    }
}

impl<Msg> SnackBar<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let measured =
            frus_text::measure_resolved(&self.text, &content_style(self.content_text_style, theme));
        let action_w = self.action.as_ref().map_or(0.0, |(label, _)| {
            action_width(label, &action_style(self.action_text_style, theme)) + ACTION_GAP
        });
        let mut style = Style {
            width: Dimension::Length((measured.width + PAD_X * 2.0 + ACCENT + action_w).ceil()),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).max(ACTION_H).ceil()),
            ..Default::default()
        };
        // With an action: place it on the right, vertically centred.
        if !self.children.is_empty() {
            style.justify = Justify::End;
            style.align = Align::Center;
            style.padding = Insets::new(0.0, PAD_X, 0.0, 0.0);
        }
        style
    }

    fn accent(&self, theme: &Theme) -> Color {
        match self.kind {
            SnackBarKind::Info => theme.primary,
            SnackBarKind::Success => Color::rgb8(70, 190, 120),
            SnackBarKind::Error => Color::rgb8(210, 96, 96),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for SnackBar<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Shadow + card.
        scene.shadow(
            Rect::new(
                bounds.x - 8.0,
                bounds.y - 4.0,
                bounds.width + 16.0,
                bounds.height + 16.0,
            ),
            theme.scheme.shadow.with_alpha(0.3).fade(o),
            theme.radius + 8.0,
            8.0,
        );
        scene.draw_rect(
            bounds,
            theme.surface.fade(o),
            theme.radius,
            1.0,
            theme.border.fade(o),
        );
        // Accent bar on the left.
        scene.draw_rect(
            Rect::new(bounds.x, bounds.y, ACCENT, bounds.height),
            self.accent(theme).fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
        scene.text(
            Point::new(bounds.x + ACCENT + PAD_X, bounds.y + PAD_Y),
            self.text.clone(),
            &content_style(self.content_text_style, Some(theme)),
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A notification's action button (uppercased text, accent color), clickable.
struct ActionButton<Msg> {
    label: String,
    text_style: Option<TextStyle>,
    message: Msg,
}

impl<Msg> ActionButton<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let style = action_style(self.text_style, theme);
        Style {
            width: Dimension::Length(action_width(&self.label, &style)),
            height: Dimension::Length(frus_text::line_box(ACTION_H, &style, 0.0)),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for ActionButton<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Hover/focus background (a baked state layer: invisible at rest, tinted on interaction).
        let bg = theme.state_layer(theme.surface, theme.primary, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        let style = action_style(self.text_style, Some(theme));
        let w = frus_text::measure_resolved(&self.label, &style).width;
        scene.text(
            Point::new(
                bounds.x + (bounds.width - w) * 0.5,
                bounds.y + (bounds.height - style.line_height()) * 0.5,
            ),
            self.label.clone(),
            &style,
            theme.primary.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }

    fn semantics(&self) -> Option<SemanticsProperties> {
        Some(
            SemanticsProperties::new(Role::Button)
                .label(self.label.clone())
                .clickable(),
        )
    }
}

/// A **notification queue** — pure, application-side (in the spirit of [`crate::form::Form`]).
///
/// Only one notification is visible at a time; the others wait. The application calls
/// [`tick`](Self::tick) on every frame (with the elapsed time) to **expire** the current
/// notification and present the next — Material-style auto-dismiss with no timer on the
/// widget side. [`dismiss`](Self::dismiss) closes the current one immediately (a click on
/// the action or the cross). Generic over the payload `T` (at minimum the text; often the
/// kind and the action message too).
pub struct SnackBarQueue<T> {
    /// `(payload, seconds left, leaving)`; the front one is the notification on display.
    /// The "leaving" flag lets the host play an **exit transition** (a fade) before the
    /// removal (see [`start_leaving`](Self::start_leaving) / [`is_leaving`](Self::is_leaving)).
    items: VecDeque<(T, f32, bool)>,
}

impl<T> Default for SnackBarQueue<T> {
    fn default() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }
}

impl<T> SnackBarQueue<T> {
    /// An empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a notification that will stay visible `seconds` seconds once **at the front**.
    pub fn push(&mut self, item: T, seconds: f32) {
        self.items.push_back((item, seconds.max(0.0), false));
    }

    /// The notification currently visible (the front of the queue), if there is one.
    pub fn current(&self) -> Option<&T> {
        self.items.front().map(|(item, _, _)| item)
    }

    /// Runs `dt` seconds down on the notification at the front; if its time is up, it is
    /// removed (the next one starts its own countdown). Returns `true` if the visible
    /// notification **changed** (an expiry) — useful to request another render.
    pub fn tick(&mut self, dt: f32) -> bool {
        let Some(front) = self.items.front_mut() else {
            return false;
        };
        front.1 -= dt;
        if front.1 <= 0.0 {
            self.items.pop_front();
            true
        } else {
            false
        }
    }

    /// Marks the current notification as **leaving**: the host can then play its exit
    /// transition (a fade) before the application removes it (via [`dismiss`](Self::dismiss)).
    pub fn start_leaving(&mut self) {
        if let Some(front) = self.items.front_mut() {
            front.2 = true;
        }
    }

    /// Is the current notification **leaving**? (a disappearing fade is under way.)
    pub fn is_leaving(&self) -> bool {
        self.items.front().is_some_and(|(_, _, leaving)| *leaving)
    }

    /// Closes the current notification at once (action, cross, end of exit); returns its payload.
    pub fn dismiss(&mut self) -> Option<T> {
        self.items.pop_front().map(|(item, _, _)| item)
    }

    /// `true` when no notification is pending.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Number of queued notifications (the visible one + those pending).
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Undo,
    }

    #[test]
    fn paints_card_accent_and_text() {
        let toast = SnackBar::<()>::new("Saved").success();
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &toast,
            Rect::new(0.0, 0.0, 160.0, 44.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        // The success accent is present, plus the text.
        let green = Color::rgb8(70, 190, 120);
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == green)));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Saved")));
    }

    #[test]
    fn action_is_clickable_and_uppercased() {
        // Without an action: no children.
        let plain = SnackBar::<Msg>::new("Item deleted");
        assert!(Widget::<Msg>::children(&plain).is_empty());
        // With an action: an uppercased button that emits the message.
        let toast = SnackBar::new("Item deleted").action("Undo", Msg::Undo);
        let kids = Widget::<Msg>::children(&toast);
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].on_click(), Some(Msg::Undo));
        assert!(kids[0].focusable());
    }

    #[test]
    fn queue_shows_one_at_a_time_and_expires() {
        let mut q: SnackBarQueue<&str> = SnackBarQueue::new();
        assert!(q.is_empty());
        q.push("first", 3.0);
        q.push("second", 3.0);
        assert_eq!(q.len(), 2);
        assert_eq!(q.current(), Some(&"first"), "the front is the visible one");
        // The countdown only touches the front.
        assert!(!q.tick(1.0));
        assert_eq!(q.current(), Some(&"first"));
        // Expiry → the next one takes over.
        assert!(q.tick(2.5), "a change at expiry");
        assert_eq!(q.current(), Some(&"second"));
        // A manual close (action or cross).
        assert_eq!(q.dismiss(), Some("second"));
        assert!(q.is_empty());
        assert!(!q.tick(1.0), "file vide : rien ne change");
    }

    #[test]
    fn leaving_phase_precedes_dismissal() {
        let mut q: SnackBarQueue<&str> = SnackBarQueue::new();
        assert!(!q.is_leaving(), "empty queue: nothing leaving");
        q.push("hello", 3.0);
        assert!(!q.is_leaving(), "on display: not leaving yet");
        // Trigger the exit (fade) without removing it right away.
        q.start_leaving();
        assert!(q.is_leaving(), "leaving");
        assert_eq!(q.current(), Some(&"hello"), "still visible while leaving");
        // Then the actual removal.
        assert_eq!(q.dismiss(), Some("hello"));
        assert!(!q.is_leaving());
        assert!(q.is_empty());
    }
}
