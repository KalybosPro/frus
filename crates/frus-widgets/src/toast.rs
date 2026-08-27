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

/// A snack bar's corner and how far off the page it sits (`snack_bar.dart:983`, `:980`).
pub const SNACK_BAR_RADIUS: f32 = 4.0;
pub const SNACK_BAR_ELEVATION: f32 = 6.0;
/// The stripe for a **success**.
///
/// Every other colour here is a role. This one cannot be: Material 3's scheme carries
/// `error` and nothing that means *it worked*, so a success has no role to reach for and
/// a framework that shipped no colour at all would be shipping no success variant. It is
/// this crate's own, and [`SnackBarTheme::success_color`] is where an application replaces
/// it.
const SUCCESS_ACCENT: Color = Color::rgb(70.0 / 255.0, 190.0 / 255.0, 120.0 / 255.0);

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
    background: Option<Color>,
    text_color: Option<Color>,
    action_text_color: Option<Color>,
    accent: Option<Color>,
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
            background: None,
            text_color: None,
            action_text_color: None,
            accent: None,
            action: None,
            children: Vec::new(),
        }
    }

    /// The bar's surface, over the theme's and the scheme's `inverse_surface`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// The message's colour, over the theme's and the scheme's `on_inverse_surface`.
    #[must_use]
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// The action's colour, over the theme's and the scheme's `inverse_primary`.
    #[must_use]
    pub fn action_text_color(mut self, color: Color) -> Self {
        self.action_text_color = Some(color);
        self.rebuild_action();
        self
    }

    /// The stripe down the leading edge, over the theme's and the kind's.
    #[must_use]
    pub fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
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

    /// The error variant.
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
                color: self.action_text_color,
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

    /// The stripe's colour: the caller's word, then the theme's, then the kind's.
    ///
    /// Two of the three kinds now name a role. The third cannot — see [`SUCCESS_ACCENT`].
    fn accent_color(&self, theme: &Theme) -> Color {
        if let Some(color) = self.accent.or(theme.widgets.snack_bar.accent_color) {
            return color;
        }
        match self.kind {
            // The accent stands on the **inverted** surface, so it is the inverted
            // accent — `primary` there is the pair the scheme guarantees nothing about.
            SnackBarKind::Info => theme.scheme.inverse_primary,
            SnackBarKind::Success => theme
                .widgets
                .snack_bar
                .success_color
                .unwrap_or(SUCCESS_ACCENT),
            SnackBarKind::Error => theme.scheme.error,
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
        let t = &theme.widgets.snack_bar;
        let radius = t.radius.unwrap_or(SNACK_BAR_RADIUS);
        let elevation = t.elevation.unwrap_or(SNACK_BAR_ELEVATION);
        // A notification is **inverted**: it is not a card on the page, it is a bar that
        // stands out from it (`snack_bar.dart:949`). The scheme has carried the pair for
        // this since it was written, and said so in its own documentation.
        let fill = self
            .background
            .or(t.background_color)
            .unwrap_or(theme.scheme.inverse_surface);
        scene.shadow(
            Rect::new(
                bounds.x - elevation,
                bounds.y - elevation * 0.5,
                bounds.width + elevation * 2.0,
                bounds.height + elevation * 2.0,
            ),
            theme.scheme.shadow.with_alpha(0.3).fade(o),
            radius + elevation,
            elevation,
        );
        // No border: the inverted surface is what separates the bar from the page, and a
        // rule round it would be edging a thing that is already distinct.
        scene.draw_rect(bounds, fill.fade(o), radius, 0.0, Color::TRANSPARENT);
        // The stripe down the leading edge is this crate's own — the reference's bar has
        // one look and no kinds — so it is drawn inside the corner rather than over it.
        scene.draw_rect(
            Rect::new(bounds.x, bounds.y, ACCENT, bounds.height),
            self.accent_color(theme).fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
        scene.text(
            Point::new(bounds.x + ACCENT + PAD_X, bounds.y + PAD_Y),
            self.text.clone(),
            &content_style(self.content_text_style, Some(theme)),
            self.text_color
                .or(t.text_color)
                .unwrap_or(theme.scheme.on_inverse_surface)
                .fade(o),
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
    /// The caller's colour, carried down from the bar so the two are said in one place.
    color: Option<Color>,
    message: Msg,
}

impl<Msg> ActionButton<Msg> {
    /// `inverse_primary` (`snack_bar.dart:965`): the accent as it must be drawn on an
    /// inverted surface.
    fn color(&self, theme: &Theme) -> Color {
        self.color
            .or(theme.widgets.snack_bar.action_text_color)
            .unwrap_or(theme.scheme.inverse_primary)
    }
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
        // Hover/focus background (a baked state layer: invisible at rest, tinted on
        // interaction). It is layered on the **inverted** surface the button stands on,
        // not on the page's — a state layer mixed from the wrong ground reads as a patch
        // of the wrong colour rather than as a tint.
        let label = self.color(theme);
        let ground = theme
            .widgets
            .snack_bar
            .background_color
            .unwrap_or(theme.scheme.inverse_surface);
        let bg = theme.state_layer(ground, label, &status);
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
            label.fade(o),
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

    /// Everything one notification paints, in one place.
    fn painted(bar: &SnackBar<()>, theme: &Theme) -> Scene {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            bar,
            Rect::new(0.0, 0.0, 160.0, 44.0),
            Status::default(),
            theme,
            &mut scene,
        );
        scene
    }

    /// The colour of the first crisp rectangle covering the whole box: the bar's surface,
    /// drawn before the stripe that sits on one edge of it.
    fn surface_of(scene: &Scene) -> Option<Color> {
        scene.primitives().iter().find_map(|p| match p {
            Primitive::Rect {
                rect, color, blur, ..
            } if *blur == 0.0 && rect.width == 160.0 => Some(*color),
            _ => None,
        })
    }

    fn text_color(scene: &Scene) -> Option<Color> {
        scene.primitives().iter().find_map(|p| match p {
            Primitive::Text { color, .. } => Some(*color),
            _ => None,
        })
    }

    #[test]
    fn paints_accent_and_text() {
        let scene = painted(&SnackBar::<()>::new("Saved").success(), &Theme::default());
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == SUCCESS_ACCENT)));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Saved")));
    }

    /// A notification is **inverted**: it does not sit on the page, it stands out from it
    /// (`snack_bar.dart:949`). The scheme has carried the pair for exactly this since it
    /// was written, saying so in its own documentation, and the bar never used it.
    #[test]
    fn a_notification_stands_out_rather_than_sitting_on_the_page() {
        let theme = Theme::default();
        let scene = painted(&SnackBar::<()>::new("Saved"), &theme);
        assert_eq!(
            surface_of(&scene),
            Some(theme.scheme.inverse_surface),
            "not a card on the page"
        );
        assert_ne!(
            theme.scheme.inverse_surface, theme.surface,
            "and the two have to be tellable apart for that to mean anything"
        );
        assert_eq!(
            text_color(&scene),
            Some(theme.scheme.on_inverse_surface),
            "with the message that is legible on it"
        );
    }

    /// The action takes the one role whose whole reason for existing is being legible on
    /// an inverted surface (`snack_bar.dart:965`).
    #[test]
    fn the_action_takes_the_inverted_accent() {
        let theme = Theme::default();
        let bar = SnackBar::<()>::new("Deleted").action("Undo", ());
        let kids = Widget::<()>::children(&bar);
        let mut scene = Scene::new();
        kids[0].paint(
            Rect::new(0.0, 0.0, 60.0, ACTION_H),
            Status::default(),
            &theme,
            &mut scene,
        );
        assert_eq!(text_color(&scene), Some(theme.scheme.inverse_primary));
        assert_ne!(
            theme.scheme.inverse_primary, theme.primary,
            "which is not the page's accent, and is the point"
        );
    }

    /// Two of the three kinds name a role now. The third cannot — Material 3 carries
    /// `error` and nothing that means *it worked* — so it keeps a colour of this crate's
    /// own, and the theme is where an application replaces it.
    #[test]
    fn the_kinds_name_a_role_wherever_one_exists() {
        let mut theme = Theme::default();
        assert_eq!(
            SnackBar::<()>::new("x").accent_color(&theme),
            theme.scheme.inverse_primary
        );
        assert_eq!(
            SnackBar::<()>::new("x").error().accent_color(&theme),
            theme.scheme.error
        );
        assert_eq!(
            SnackBar::<()>::new("x").success().accent_color(&theme),
            SUCCESS_ACCENT
        );

        let told = Color::rgb8(1, 2, 3);
        theme.widgets.snack_bar.success_color = Some(told);
        assert_eq!(
            SnackBar::<()>::new("x").success().accent_color(&theme),
            told,
            "the one colour without a role is the one a theme most needs to reach"
        );
        assert_eq!(
            SnackBar::<()>::new("x").accent(told).accent_color(&theme),
            told,
            "and the caller outranks all three kinds"
        );
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
        assert!(!q.tick(1.0), "an empty queue: nothing changes");
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
