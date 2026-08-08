//! [`Toast`]: a transient notification (a styled card). The *system* around it
//! (stacking, the auto-dismiss timer) is the application's responsibility,
//! typically through a timed `Command`.

use std::collections::VecDeque;

use frus_core::{Color, Insets, Point, Rect, Role, Scene, Semantics};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 12.0;
const SIZE: f32 = 16.0;
const ACCENT: f32 = 4.0;
/// The action button (Material's "UNDO"): font, padding and height.
const ACTION_SIZE: f32 = 14.0;
const ACTION_PAD_X: f32 = 12.0;
const ACTION_GAP: f32 = 8.0;
const ACTION_H: f32 = 32.0;

/// The nature of a notification (its accent color).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

/// A transient notification, with an optional **action** (Material Snackbar style:
/// "UNDO"). The action is a text button on the right that emits a message on click.
pub struct Toast<Msg> {
    text: String,
    kind: ToastKind,
    /// Extra width reserved for the action (0 if there is none).
    action_w: f32,
    /// Empty, or `[action button]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Toast<Msg> {
    /// Creates an informational notification.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: ToastKind::Info,
            action_w: 0.0,
            children: Vec::new(),
        }
    }

    /// The success variant.
    pub fn success(mut self) -> Self {
        self.kind = ToastKind::Success;
        self
    }

    /// Variante erreur.
    pub fn error(mut self) -> Self {
        self.kind = ToastKind::Error;
        self
    }

    /// Adds an **action button** (an uppercased label, Material style) that emits `message`
    /// on click — typically "UNDO", to undo whatever triggered the notification.
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        let label = label.into().to_uppercase();
        let width = (frus_text::measure(&label, ACTION_SIZE).width + ACTION_PAD_X * 2.0).ceil();
        self.action_w = width + ACTION_GAP;
        self.children = vec![Box::new(ActionButton {
            label,
            width,
            message,
        })];
        self
    }
}

impl<Msg> Toast<Msg> {
    fn accent(&self, theme: &Theme) -> Color {
        match self.kind {
            ToastKind::Info => theme.primary,
            ToastKind::Success => Color::rgb8(70, 190, 120),
            ToastKind::Error => Color::rgb8(210, 96, 96),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Toast<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.text, SIZE);
        let mut style = Style {
            width: Dimension::Length(
                (measured.width + PAD_X * 2.0 + ACCENT + self.action_w).ceil(),
            ),
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
            SIZE,
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
    width: f32,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for ActionButton<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(ACTION_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Hover/focus background (a baked state layer: invisible at rest, tinted on interaction).
        let bg = theme.state_layer(theme.surface, theme.primary, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        let w = frus_text::measure(&self.label, ACTION_SIZE).width;
        scene.text(
            Point::new(
                bounds.x + (bounds.width - w) * 0.5,
                bounds.y + (bounds.height - frus_text::line_height(ACTION_SIZE)) * 0.5,
            ),
            self.label.clone(),
            ACTION_SIZE,
            theme.primary.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }

    fn semantics(&self) -> Option<Semantics> {
        Some(
            Semantics::new(Role::Button)
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
pub struct SnackbarQueue<T> {
    /// `(payload, seconds left, leaving)`; the front one is the notification on display.
    /// The "leaving" flag lets the host play an **exit transition** (a fade) before the
    /// removal (see [`start_leaving`](Self::start_leaving) / [`is_leaving`](Self::is_leaving)).
    items: VecDeque<(T, f32, bool)>,
}

impl<T> Default for SnackbarQueue<T> {
    fn default() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }
}

impl<T> SnackbarQueue<T> {
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
        let toast = Toast::<()>::new("Saved").success();
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
        let plain = Toast::<Msg>::new("Item deleted");
        assert!(Widget::<Msg>::children(&plain).is_empty());
        // With an action: an uppercased button that emits the message.
        let toast = Toast::new("Item deleted").action("Undo", Msg::Undo);
        let kids = Widget::<Msg>::children(&toast);
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].on_click(), Some(Msg::Undo));
        assert!(kids[0].focusable());
    }

    #[test]
    fn queue_shows_one_at_a_time_and_expires() {
        let mut q: SnackbarQueue<&str> = SnackbarQueue::new();
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
        let mut q: SnackbarQueue<&str> = SnackbarQueue::new();
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
