//! [`SnackBar`]: a transient notification (a styled card). The *system* around it
//! (stacking, the auto-dismiss timer) is the application's responsibility,
//! typically through a timed `Command`.

use std::collections::VecDeque;

use frus_core::{
    BorderRadius, Color, Insets, Point, Rect, ResolvedTextStyle, Role, Scene, SemanticsProperties,
    ShapeBorder, TextStyle,
};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::iconbutton::{ICON_BUTTON_ICON_SIZE, ICON_BUTTON_SIZE};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The grid the icon paths are drawn on, and therefore what a glyph is scaled from.
const ICON_GRID: f32 = 24.0;

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

/// A bar's horizontal padding, which is **not the same for both behaviours**
/// (`snack_bar.dart:687`): a fixed bar spans the page and holds its text further in; a
/// floating one is already held off the edges by its own margin.
const PAD_X_FIXED: f32 = 24.0;
const PAD_X_FLOATING: f32 = 16.0;
/// What a **floating** bar keeps clear of the page (`snack_bar.dart:989`). A fixed one
/// keeps nothing clear: it is fixed to the edge, which is what fixed means.
const INSET_PADDING: Insets = Insets::new(5.0, 15.0, 10.0, 15.0);
const PAD_Y: f32 = 12.0;
const ACCENT: f32 = 4.0;
/// The action button (Material's "UNDO"): padding and height.
const ACTION_PAD_X: f32 = 12.0;
const ACTION_GAP: f32 = 8.0;
const ACTION_H: f32 = 32.0;
/// The air either side of the close icon. The reference's is a **twelfth** of the bar's
/// horizontal padding (`snack_bar.dart:698`), which is as near to nothing as a margin
/// gets — the cross is meant to sit at the very end of the bar. It follows the padding,
/// so it is not the same number for both behaviours either.
fn close_margin(pad_x: f32) -> f32 {
    pad_x / 12.0
}
/// The default label a reader hears on it, from the table in force
/// (`snack_bar.dart:709`) — see [`crate::localizations`]. A caller may still say its own,
/// which is what [`SnackBar::close_icon_label`] is for.
fn close_label() -> String {
    crate::localizations::of().close_button_label().to_string()
}

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

/// **Where a snack bar sits, and therefore what it looks like** (`snack_bar_theme.dart:27`).
///
/// The reference treats this as one question with consequences rather than a bag of
/// settings: a bar fixed to the bottom of the page and a bar floating above it differ in
/// their corners, their padding and the room they keep clear, and they differ that way
/// because of where they are.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum SnackBarBehavior {
    /// **Fixed to the bottom of the page**: square corners, flush to the edges, its text
    /// held further in. The reference's default (`snack_bar.dart:986`).
    #[default]
    Fixed,
    /// **Floating above the page**: rounded, held clear of the edges by its own margin,
    /// and drawn over everything rather than pushing anything up.
    Floating,
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
    /// Where it sits; `None` follows the theme, which follows the reference's `Fixed`.
    behavior: Option<SnackBarBehavior>,
    /// A floating bar's own width, over the room it is given. Floating only.
    width: Option<f32>,
    /// A floating bar's own margin, over `insetPadding`. Floating only.
    margin: Option<Insets>,
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
    /// What a click on the cross emits, when there is one. See [`SnackBar::close_icon`].
    close: Option<Msg>,
    close_icon_color: Option<Color>,
    close_label: Option<String>,
    /// The trailing controls, in order: the action, then the cross.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SnackBar<Msg> {
    /// Creates an informational notification.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SnackBarKind::Info,
            behavior: None,
            width: None,
            margin: None,
            content_text_style: None,
            action_text_style: None,
            background: None,
            text_color: None,
            action_text_color: None,
            accent: None,
            action: None,
            close: None,
            close_icon_color: None,
            close_label: None,
            children: Vec::new(),
        }
    }

    /// **Where the bar sits** (`snack_bar.dart:405`), over the theme's answer and the
    /// reference's `Fixed`.
    ///
    /// It is one question with consequences, not a setting: see [`SnackBarBehavior`].
    #[must_use]
    pub fn behavior(mut self, behavior: SnackBarBehavior) -> Self {
        self.behavior = Some(behavior);
        self
    }

    /// **A floating bar's own width** (`snack_bar.dart:329`), instead of the room it is
    /// given less its margin.
    ///
    /// Floating only, as in the reference: a fixed bar spans the page by definition, and
    /// a width would be a contradiction rather than an override. Ignored under `Fixed`.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// **A floating bar's own margin** (`snack_bar.dart:347`), over `insetPadding`.
    ///
    /// Floating only, for the same reason as [`SnackBar::width`]: what a fixed bar keeps
    /// clear of the page is nothing, which is what fixed means. Ignored under `Fixed`.
    #[must_use]
    pub fn margin(mut self, margin: Insets) -> Self {
        self.margin = Some(margin);
        self
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
        self.rebuild_controls();
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
        self.rebuild_controls();
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
        self.rebuild_controls();
        self
    }

    /// Carries the current action *and the current style* into the child.
    ///
    /// **The cross at the end of the bar** (`snack_bar.dart:700`), emitting `message`.
    ///
    /// The reference's property is a `bool`, because there a `ScaffoldMessenger` owns the
    /// bar and the button can call `hideCurrentSnackBar` on it. Here the application owns
    /// the queue — [`SnackBarQueue`] is deliberately application-side — so a bool would
    /// draw a cross that does nothing, and a button that cannot say what it does is worse
    /// than no button. It takes the message instead.
    #[must_use]
    pub fn close_icon(mut self, message: Msg) -> Self {
        self.close = Some(message);
        self.rebuild_controls();
        self
    }

    /// The cross's colour, over the theme's and the scheme's `on_inverse_surface`
    /// (`snack_bar.dart:995`).
    #[must_use]
    pub fn close_icon_color(mut self, color: Color) -> Self {
        self.close_icon_color = Some(color);
        self.rebuild_controls();
        self
    }

    /// What a reader hears on the cross, over the word the table in force would give
    /// (`snack_bar.dart:709`). See [`crate::localizations`].
    #[must_use]
    pub fn close_icon_label(mut self, label: impl Into<String>) -> Self {
        self.close_label = Some(label.into());
        self.rebuild_controls();
        self
    }

    /// Called by every builder that touches them, so the builders are order-independent:
    /// `.action(…)` then `.action_text_style(…)` and the reverse describe the same
    /// notification, which a caller is entitled to assume and would otherwise have to
    /// discover.
    fn rebuild_controls(&mut self) {
        self.children = Vec::new();
        if let Some((label, message)) = &self.action {
            self.children.push(Box::new(ActionButton {
                label: label.clone(),
                text_style: self.action_text_style,
                color: self.action_text_color,
                message: message.clone(),
            }));
        }
        // **After** the action, which is the order the reference builds them in
        // (`snack_bar.dart:742`): the cross is the last thing on the line, so the action
        // keeps the place a reader looks for it whether or not there is a cross.
        if let Some(message) = &self.close {
            self.children.push(Box::new(CloseButton {
                color: self.close_icon_color,
                label: self.close_label.clone().unwrap_or_else(close_label),
                message: message.clone(),
            }));
        }
    }
}

impl<Msg> SnackBar<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let measured =
            frus_text::measure_resolved(&self.text, &content_style(self.content_text_style, theme));
        let action_w = self.action.as_ref().map_or(0.0, |(label, _)| {
            action_width(label, &action_style(self.action_text_style, theme)) + ACTION_GAP
        });
        let pad_x = self.pad_x(theme);
        let close_margin = close_margin(pad_x);
        let close_w = self
            .close
            .as_ref()
            .map_or(0.0, |_| ICON_BUTTON_SIZE + close_margin * 2.0);
        // The bar is at least as tall as the tallest thing in it. A cross is a 40-pixel
        // box where the action is 32, so a bar sized for the action alone would have cut
        // the top and bottom off it.
        let floor = if self.close.is_some() {
            ICON_BUTTON_SIZE
        } else {
            ACTION_H
        };
        // A floating bar may be given a width of its own (`snack_bar.dart:329`); a fixed
        // one may not, and says so by ignoring it rather than by asserting, this being a
        // builder and not a debug-mode framework.
        let asked = match self.placing(theme) {
            SnackBarBehavior::Floating => {
                self.width.or(theme.and_then(|t| t.widgets.snack_bar.width))
            }
            SnackBarBehavior::Fixed => None,
        };
        let mut style = Style {
            width: Dimension::Length(
                asked
                    .unwrap_or((measured.width + pad_x * 2.0 + ACCENT + action_w + close_w).ceil()),
            ),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).max(floor).ceil()),
            // What it keeps clear of the page belongs to the **bar**, which is where the
            // reference keeps it (`snack_bar.dart:823`) — not to whatever is hosting it.
            margin: self.inset(theme),
            ..Default::default()
        };
        // With controls: place them on the right, vertically centred, the cross tight
        // against the end of the bar.
        if !self.children.is_empty() {
            style.justify = Justify::End;
            style.align = Align::Center;
            style.gap = close_margin;
            style.padding = Insets::new(
                0.0,
                if self.close.is_some() {
                    close_margin
                } else {
                    pad_x
                },
                0.0,
                0.0,
            );
        }
        style
    }

    /// Where this bar sits: its own word, then the theme's, then the reference's
    /// `Fixed` (`snack_bar.dart:986`).
    fn placing(&self, theme: Option<&Theme>) -> SnackBarBehavior {
        self.behavior
            .or(theme.and_then(|t| t.widgets.snack_bar.behavior))
            .unwrap_or_default()
    }

    /// The horizontal padding that placing implies (`snack_bar.dart:687`).
    fn pad_x(&self, theme: Option<&Theme>) -> f32 {
        match self.placing(theme) {
            SnackBarBehavior::Fixed => PAD_X_FIXED,
            SnackBarBehavior::Floating => PAD_X_FLOATING,
        }
    }

    /// What it keeps clear of the page: nothing when fixed; its own word, the theme's,
    /// then `insetPadding` when floating (`snack_bar.dart:989`).
    fn inset(&self, theme: Option<&Theme>) -> Insets {
        match self.placing(theme) {
            SnackBarBehavior::Fixed => Insets::ZERO,
            SnackBarBehavior::Floating => self
                .margin
                .or(theme.and_then(|t| t.widgets.snack_bar.inset_padding))
                .unwrap_or(INSET_PADDING),
        }
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
        // **A fixed bar has no shape.** The reference passes `shape` at all only when the
        // bar is floating (`snack_bar.dart:798`), and a `Material` with no shape has
        // square corners — a bar flush against three edges of the page has nothing to
        // round, and rounding it would leave four slivers of page showing through.
        let shape = crate::resolve_shape(
            t.shape,
            None,
            t.radius.map(BorderRadius::uniform),
            ShapeBorder::rounded(match self.placing(Some(theme)) {
                SnackBarBehavior::Fixed => 0.0,
                SnackBarBehavior::Floating => SNACK_BAR_RADIUS,
            }),
        );
        // The shadow is a rectangle whatever the bar is.
        let radius = shape
            .as_rounded(bounds)
            .map(|(_, radius)| radius)
            .unwrap_or(BorderRadius::ZERO);
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
            radius.inflate(elevation),
            elevation,
        );
        // No border: the inverted surface is what separates the bar from the page, and a
        // rule round it would be edging a thing that is already distinct.
        scene.draw_shape(bounds, shape, fill.fade(o));
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
            Point::new(
                bounds.x + ACCENT + self.pad_x(Some(theme)),
                bounds.y + PAD_Y,
            ),
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

/// A notification's **close** button: the cross at the end of the bar
/// (`snack_bar.dart:700`).
struct CloseButton<Msg> {
    /// The caller's colour, carried down from the bar so the two are said in one place.
    color: Option<Color>,
    label: String,
    message: Msg,
}

impl<Msg> CloseButton<Msg> {
    /// `on_inverse_surface` (`snack_bar.dart:995`): the cross stands on the inverted
    /// surface, so it takes the ink that surface guarantees is legible.
    fn color(&self, theme: &Theme) -> Color {
        self.color
            .or(theme.widgets.snack_bar.close_icon_color)
            .unwrap_or(theme.scheme.on_inverse_surface)
    }
}

impl<Msg: Clone> Widget<Msg> for CloseButton<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(ICON_BUTTON_SIZE),
            height: Dimension::Length(ICON_BUTTON_SIZE),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let ink = self.color(theme);
        // Grounded on the **inverted** surface the bar painted, not on the page's, for
        // the reason [`ActionButton`] gives: a state layer mixed from the wrong ground
        // reads as a patch of the wrong colour rather than as a tint. It is also why this
        // is not a plain [`crate::IconButton`], which grounds a standard one on nothing.
        let ground = theme
            .widgets
            .snack_bar
            .background_color
            .unwrap_or(theme.scheme.inverse_surface);
        let bg = theme.state_layer(ground, ink, &status);
        scene.draw_rect(
            bounds,
            bg.fade(o),
            bounds.height.min(bounds.width) * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        let size = ICON_BUTTON_ICON_SIZE;
        let path = crate::Icons::Close
            .path()
            .scaled(size / ICON_GRID)
            .translated(
                bounds.x + (bounds.width - size) * 0.5,
                bounds.y + (bounds.height - size) * 0.5,
            );
        scene.fill_path(&path, ink.fade(o));
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
        Dismiss,
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

    /// The corner of the first crisp rectangle covering the whole box.
    fn corner_of(scene: &Scene) -> Option<f32> {
        scene.primitives().iter().find_map(|p| match p {
            Primitive::Rect {
                rect, radius, blur, ..
            } if *blur == 0.0 && rect.width == 160.0 => Some(radius.top_left),
            _ => None,
        })
    }

    /// Where the message starts, horizontally.
    fn text_x(scene: &Scene) -> Option<f32> {
        scene.primitives().iter().find_map(|p| match p {
            Primitive::Text { position, .. } => Some(position.x),
            _ => None,
        })
    }

    /// **A bar is fixed unless it is told otherwise** (`snack_bar.dart:986`), and a fixed
    /// bar has **no corners**: the reference passes a shape at all only when the bar is
    /// floating (`:798`), and a bar flush against three edges of the page has nothing to
    /// round — rounding it would leave four slivers of page showing through.
    ///
    /// This rounded every bar, and held its text in by a floating bar's 16 whichever it
    /// was (`:687`).
    #[test]
    fn a_bar_is_fixed_by_default_and_has_no_corners() {
        let theme = Theme::default();
        let bar = SnackBar::<Msg>::new("Saved");
        let scene = painted(&SnackBar::<()>::new("Saved"), &theme);
        assert_eq!(corner_of(&scene), Some(0.0), "square, being flush");
        assert_eq!(
            text_x(&scene),
            Some(ACCENT + PAD_X_FIXED),
            "and its text held in by a fixed bar's padding"
        );
        assert_eq!(
            Widget::<Msg>::style_themed(&bar, &theme).margin,
            Insets::ZERO,
            "a fixed bar keeps nothing clear of the page: that is what fixed means"
        );
    }

    /// **A floating bar rounds, and keeps clear of the page** by its own margin
    /// (`snack_bar.dart:989`, `:823`) — which belongs to the bar and not to whatever is
    /// hosting it.
    #[test]
    fn a_floating_bar_rounds_and_keeps_clear_of_the_page() {
        let theme = Theme::default();
        let scene = painted(
            &SnackBar::<()>::new("Saved").behavior(SnackBarBehavior::Floating),
            &theme,
        );
        assert_eq!(corner_of(&scene), Some(SNACK_BAR_RADIUS));
        assert_eq!(
            text_x(&scene),
            Some(ACCENT + PAD_X_FLOATING),
            "and its text is held in less, the margin having held it in already"
        );

        let bar = SnackBar::<Msg>::new("Saved").behavior(SnackBarBehavior::Floating);
        assert_eq!(
            Widget::<Msg>::style_themed(&bar, &theme).margin,
            INSET_PADDING
        );
        let own = Insets::uniform(4.0);
        assert_eq!(
            Widget::<Msg>::style_themed(
                &SnackBar::<Msg>::new("Saved")
                    .behavior(SnackBarBehavior::Floating)
                    .margin(own),
                &theme
            )
            .margin,
            own,
            "and a caller may say what it keeps clear"
        );
    }

    /// **A width, and a margin, are a floating bar's alone** (`snack_bar.dart:678`): a
    /// fixed bar spans the page by definition, so both would contradict the behaviour
    /// rather than override it. The reference asserts; a builder can only ignore.
    #[test]
    fn a_width_is_a_floating_bar_s_alone() {
        let theme = Theme::default();
        let floating = SnackBar::<Msg>::new("Saved")
            .behavior(SnackBarBehavior::Floating)
            .width(300.0);
        assert_eq!(width_of(&floating), 300.0);

        let fixed = SnackBar::<Msg>::new("Saved")
            .width(300.0)
            .margin(Insets::uniform(9.0));
        assert_ne!(
            width_of(&fixed),
            300.0,
            "a fixed bar spans what it is given"
        );
        assert_eq!(
            Widget::<Msg>::style_themed(&fixed, &theme).margin,
            Insets::ZERO,
            "and keeps nothing clear, whatever it was told"
        );
    }

    /// And the **theme** answers for a whole application's bars, as it does for every
    /// other widget here: the caller's word, then the theme's, then the reference's.
    #[test]
    fn a_theme_places_every_bar_at_once() {
        let mut theme = Theme::default();
        theme.widgets.snack_bar.behavior = Some(SnackBarBehavior::Floating);
        let scene = {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &SnackBar::<()>::new("Saved"),
                Rect::new(0.0, 0.0, 160.0, 44.0),
                Status::default(),
                &theme,
                &mut scene,
            );
            scene
        };
        assert_eq!(
            corner_of(&scene),
            Some(SNACK_BAR_RADIUS),
            "the theme's word"
        );
        assert_eq!(
            Widget::<Msg>::style_themed(&SnackBar::<Msg>::new("Saved"), &theme).margin,
            INSET_PADDING
        );
        assert_eq!(
            Widget::<Msg>::style_themed(
                &SnackBar::<Msg>::new("Saved").behavior(SnackBarBehavior::Fixed),
                &theme
            )
            .margin,
            Insets::ZERO,
            "and one bar may still answer for itself"
        );
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

    /// The width a notification asks for.
    fn width_of(bar: &SnackBar<Msg>) -> f32 {
        match Widget::<Msg>::style_themed(bar, &Theme::default()).width {
            Dimension::Length(w) => w,
            other => panic!("a notification names its width, got {other:?}"),
        }
    }

    /// The height a notification asks for.
    fn height_of(bar: &SnackBar<Msg>) -> f32 {
        match Widget::<Msg>::style_themed(bar, &Theme::default()).height {
            Dimension::Length(h) => h,
            other => panic!("a notification names its height, got {other:?}"),
        }
    }

    /// Paints one of a notification's trailing controls, in its own box.
    fn control(bar: &SnackBar<Msg>, index: usize, status: Status, theme: &Theme) -> Scene {
        let mut scene = Scene::new();
        Widget::<Msg>::children(bar)[index].paint(
            Rect::new(0.0, 0.0, ICON_BUTTON_SIZE, ICON_BUTTON_SIZE),
            status,
            theme,
            &mut scene,
        );
        scene
    }

    /// **A close icon is a button that says what it does** (`snack_bar.dart:700`).
    ///
    /// The reference's property is a `bool`, because a `ScaffoldMessenger` owns the bar
    /// there and the cross can hide it. Here the application owns the queue, so a bool
    /// would draw a cross that does nothing.
    #[test]
    fn a_close_icon_is_a_button_that_says_what_it_does() {
        let bar = SnackBar::new("Message archived").close_icon(Msg::Dismiss);
        let children = Widget::<Msg>::children(&bar);
        assert_eq!(children.len(), 1, "the cross, and nothing else");
        assert_eq!(children[0].on_click(), Some(Msg::Dismiss));
        assert!(children[0].focusable(), "and the keyboard can reach it");

        let heard = |bar: &SnackBar<Msg>| {
            Widget::<Msg>::children(bar)[0]
                .semantics()
                .and_then(|s| s.label)
                .expect("a cross with no name is a cross nobody can use")
        };
        assert_eq!(heard(&bar), close_label());
        assert_eq!(
            heard(
                &SnackBar::new("x")
                    .close_icon(Msg::Dismiss)
                    .close_icon_label("Fermer")
            ),
            "Fermer",
            "the caller is the only one who can translate it"
        );
    }

    /// **The cross comes after the action** (`snack_bar.dart:742`), so the action keeps
    /// the place a reader looks for it whether or not there is a cross.
    #[test]
    fn the_cross_comes_after_the_action() {
        let bar = SnackBar::new("Message archived")
            .close_icon(Msg::Dismiss)
            .action("Undo", Msg::Undo);
        let children = Widget::<Msg>::children(&bar);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].on_click(), Some(Msg::Undo));
        assert_eq!(children[1].on_click(), Some(Msg::Dismiss));
    }

    /// And the bar makes room for it: wider by the cross's box, and **at least as tall**
    /// as it. A bar sized for the action alone is 32 high, and the cross is 40.
    #[test]
    fn a_bar_with_a_cross_makes_room_for_it() {
        let bare = SnackBar::new("Saved");
        let crossed = SnackBar::new("Saved").close_icon(Msg::Dismiss);
        assert!(
            (width_of(&crossed)
                - width_of(&bare)
                - (ICON_BUTTON_SIZE + close_margin(PAD_X_FIXED) * 2.0))
                .abs()
                < 1.0,
            "{} against {}",
            width_of(&crossed),
            width_of(&bare)
        );
        // Asked with type small enough that the message alone does not already make the
        // bar tall enough, which is what makes the floor a floor rather than a comment.
        let small = TextStyle {
            size: Some(8.0),
            ..Default::default()
        };
        let tiny = SnackBar::new("Saved")
            .close_icon(Msg::Dismiss)
            .content_text_style(small);
        assert!(
            height_of(&SnackBar::new("Saved").content_text_style(small)) < ICON_BUTTON_SIZE,
            "the premise: a bar of small type is shorter than a cross"
        );
        assert!(
            height_of(&tiny) >= ICON_BUTTON_SIZE,
            "the cross would have been cut off: {}",
            height_of(&tiny)
        );
    }

    /// The cross takes the ink that is legible on the bar (`snack_bar.dart:995`), and the
    /// caller and the theme each outrank the scheme.
    #[test]
    fn the_cross_takes_the_ink_that_is_legible_on_the_bar() {
        let mut theme = Theme::default();
        let ink = |bar: &SnackBar<Msg>, theme: &Theme| {
            control(bar, 0, Status::default(), theme)
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Path { fill, .. } => *fill,
                    _ => None,
                })
                .expect("the cross is drawn")
        };
        let bar = SnackBar::new("x").close_icon(Msg::Dismiss);
        assert_eq!(ink(&bar, &theme), theme.scheme.on_inverse_surface);

        theme.widgets.snack_bar.close_icon_color = Some(Color::rgb8(1, 2, 3));
        assert_eq!(ink(&bar, &theme), Color::rgb8(1, 2, 3));

        let told = SnackBar::new("x")
            .close_icon(Msg::Dismiss)
            .close_icon_color(Color::rgb8(4, 5, 6));
        assert_eq!(ink(&told, &theme), Color::rgb8(4, 5, 6));
    }

    /// **Its state layer is grounded on the bar**, not on the page. A layer mixed from
    /// the wrong ground reads as a patch of the wrong colour rather than as a tint —
    /// which is also why this is not a plain `IconButton`, whose standard variant grounds
    /// on nothing at all.
    #[test]
    fn the_cross_grounds_its_state_layer_on_the_bar() {
        let theme = Theme::default();
        let bar = SnackBar::new("x").close_icon(Msg::Dismiss);
        let hovered = Status {
            opacity: 1.0,
            hover_progress: 1.0,
            ..Default::default()
        };
        let fill = control(&bar, 0, hovered, &theme)
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .expect("a hovered cross lights");
        assert_eq!(fill.a, 1.0, "resolved here, not handed over as an alpha");
        assert_eq!(
            fill,
            theme.state_layer(
                theme.scheme.inverse_surface,
                theme.scheme.on_inverse_surface,
                &hovered
            )
        );
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
