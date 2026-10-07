//! [`Checkbox`]: a **controlled** checkbox, its state coming from the application — the
//! reference's `Checkbox` (`material/checkbox.dart`), with an optional label beside it.

use frus_core::{
    BorderSide, Color, Path, Point, Rect, ResolvedTextStyle, Scene, Stroke, TextStyle,
};
use frus_layout::{Dimension, Style};

use crate::disabled::{disabled_content, disabled_mark};
use crate::interaction::Status;
use crate::theme::{TapTarget, Theme};
use crate::widget::Widget;
use crate::widgetstate::{WidgetState, WidgetStateProperty, WidgetStates};

/// The box's side (`checkbox.dart:405`).
const EDGE: f32 = 18.0;
/// The outline's and the mark's stroke (`checkbox.dart:651`).
const STROKE: f32 = 2.0;
/// The box's corner (`checkbox.dart:1047`).
const RADIUS: f32 = 2.0;
/// The halo's radius under a pointer, the keyboard or a finger (`checkbox.dart:1037`).
const SPLASH: f32 = 20.0;
/// How long a tick takes to draw itself (the reference's toggle animation).
const TOGGLE_SECONDS: f32 = 0.2;

/// A checkbox, with an optional label.
///
/// The box is the reference's: 18 px with a 2 px corner, centred in the room it reserves
/// for a finger (48 px at the standard density), its outline `on_surface_variant` at rest
/// and `on_surface` under a pointer, the keyboard or a finger, filled in `primary` with an
/// `on_primary` tick drawn as a stroke, or a dash when partly ticked, and a halo behind it
/// under a pointer, the keyboard or a finger. Ticking it fills the box and draws the tick
/// over a fifth of a second.
///
/// The label is this framework's: the reference puts words beside a checkbox with a list
/// tile. It follows the box's room.
pub struct Checkbox<Msg = crate::callback::Callback> {
    /// On, off, or **partly** on; see [`Checkbox::maybe`].
    value: Option<bool>,
    /// Whether a click may land on the partly-on answer.
    tristate: bool,
    label: Option<String>,
    size: f32,
    enabled: bool,
    error: bool,
    semantic_label: Option<String>,
    fill_color: Option<Color>,
    check_color: Option<Color>,
    border_color: Option<Color>,
    active_border_color: Option<Color>,
    overlay_color: Option<WidgetStateProperty<Color>>,
    splash_radius: Option<f32>,
    tap_target: Option<TapTarget>,
    visual_density: Option<crate::VisualDensity>,
    radius: Option<f32>,
    label_color: Option<Color>,
    on_toggle: Option<Box<dyn Fn(bool) -> Msg>>,
    on_change: Option<Box<dyn Fn(Option<bool>) -> Msg>>,
}

impl<Msg> Checkbox<Msg> {
    /// Creates a checkbox whose checked state is supplied.
    pub fn new(checked: bool) -> Self {
        Self {
            value: Some(checked),
            tristate: false,
            label: None,
            size: 16.0,
            enabled: true,
            error: false,
            semantic_label: None,
            fill_color: None,
            check_color: None,
            border_color: None,
            active_border_color: None,
            overlay_color: None,
            splash_radius: None,
            tap_target: None,
            visual_density: None,
            radius: None,
            label_color: None,
            on_toggle: None,
            on_change: None,
        }
    }

    /// A checkbox with **three** answers: on, off, and partly on.
    ///
    /// `None` is the third, and it is an answer rather than a missing one. A "select
    /// all" above five rows of which three are ticked is not unchecked — saying so tells
    /// the reader something false, and a screen reader is told `mixed` for the same
    /// reason.
    ///
    /// A click cycles off → on → partly on → off, which is the reference's order. Pair it
    /// with [`on_change`](Checkbox::on_change), since [`on_toggle`](Checkbox::on_toggle)
    /// has no way to say the third answer.
    ///
    /// ```
    /// # use frus_widgets::Checkbox;
    /// # #[derive(Clone)] enum Msg { All(Option<bool>) }
    /// # let (done, total) = (3usize, 5usize);
    /// let all = match done {
    ///     0 => Some(false),
    ///     n if n == total => Some(true),
    ///     _ => None,
    /// };
    /// Checkbox::maybe(all).label("Select all").on_change(Msg::All);
    /// ```
    pub fn maybe(value: Option<bool>) -> Self {
        let mut checkbox = Self::new(false);
        checkbox.value = value;
        checkbox.tristate = true;
        checkbox
    }

    /// The box's fill when it is **ticked**; the theme's, then `primary`.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// The tick drawn on that fill; the theme's, then `on_primary`.
    pub fn check_color(mut self, color: Color) -> Self {
        self.check_color = Some(color);
        self
    }

    /// The outline when the box is **not** ticked and at rest.
    ///
    /// Set on its own it also becomes the colour under a pointer or focus, unless
    /// [`active_border_color`](Checkbox::active_border_color) says otherwise: a caller
    /// who names one outline colour means the outline, not half of it.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// The outline under a pointer, a finger or focus.
    pub fn active_border_color(mut self, color: Color) -> Self {
        self.active_border_color = Some(color);
        self
    }

    /// **The halo** under a pointer, the keyboard or a finger, state by state — the
    /// reference's `overlayColor`. What it does not say, the theme's and then the
    /// reference's answer.
    #[must_use]
    pub fn overlay_color(mut self, overlay: WidgetStateProperty<Color>) -> Self {
        self.overlay_color = Some(overlay);
        self
    }

    /// That halo's radius. Unset, the theme's, then 20.
    #[must_use]
    pub fn splash_radius(mut self, radius: f32) -> Self {
        self.splash_radius = Some(radius);
        self
    }

    /// **How compact it is.** Unset, the theme's checkbox density, then the standard one —
    /// a checkbox keeps its 48 px on every platform unless told, as the reference's does
    /// (`checkbox.dart:1043`).
    #[must_use]
    pub fn visual_density(mut self, density: crate::VisualDensity) -> Self {
        self.visual_density = Some(density);
        self
    }

    /// The box's corner radius. Unset, the theme's, then 2.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    /// The label's colour; the theme's `on_surface` otherwise.
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = Some(color);
        self
    }

    /// **In error**: the outline, the fill and the halo take the `error` colours, as the
    /// reference's `isError` does — a required box left unticked in a form.
    #[must_use]
    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    /// **What a screen reader calls it** when it has no label to read.
    #[must_use]
    pub fn semantic_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_label = Some(label.into());
        self
    }

    /// The corner radius actually used.
    fn corner(&self, theme: &Theme) -> f32 {
        self.radius
            .or(theme.widgets.checkbox.radius)
            .unwrap_or(RADIUS)
    }

    /// Adds a label after it.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Whether the box can be ticked. Disabled it is **inert** — no message, out of the
    /// tab order, announced as unavailable — and it still shows whether it is ticked,
    /// because read-only is not invisible.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A closure producing a message from the new state, checked or not.
    pub fn on_toggle<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_toggle: impl Fn(bool) -> R + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(crate::callback::handler1(on_toggle)));
        self
    }

    /// A closure producing a message from the new state, **including** the partly-on
    /// one. What [`Checkbox::maybe`] wants; it wins over
    /// [`on_toggle`](Checkbox::on_toggle) when both are given.
    pub fn on_change<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        on_change: impl Fn(Option<bool>) -> R + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(crate::callback::handler1(on_change)));
        self
    }

    /// The state a click moves to. Three-way it is the reference's cycle.
    fn next(&self) -> Option<bool> {
        if self.tristate {
            match self.value {
                Some(false) => Some(true),
                Some(true) => None,
                None => Some(false),
            }
        } else {
            Some(self.value != Some(true))
        }
    }

    /// **How much room it reserves for a finger** ([`TapTarget`]). Unset, the theme's
    /// answer, which is [`Padded`](TapTarget::Padded) — 48 pixels either way, whatever this
    /// control paints in the middle of it.
    pub fn tap_target(mut self, target: TapTarget) -> Self {
        self.tap_target = Some(target);
        self
    }

    /// The label's style, **resolved once** so that the number the box is measured with is
    /// the number the glyphs are drawn at.
    fn label_style(&self) -> ResolvedTextStyle {
        TextStyle::new(self.size).resolved()
    }

    /// The square the box sits in: 48 px padded or 40 shrink-wrapped, moved by the density
    /// (`checkbox.dart:516`).
    fn square(&self, theme: &Theme) -> f32 {
        let target = self
            .tap_target
            .or(theme.widgets.checkbox.tap_target)
            .unwrap_or(theme.tap_target);
        let side = match target {
            TapTarget::Padded => crate::MIN_TAP_TARGET,
            TapTarget::ShrinkWrap => crate::MIN_TAP_TARGET - 8.0,
        };
        let density = self
            .visual_density
            .or(theme.widgets.checkbox.visual_density)
            .unwrap_or(crate::VisualDensity::STANDARD);
        (side + density.base_size_adjustment().1).max(EDGE)
    }

    fn label_width(&self) -> f32 {
        match &self.label {
            Some(text) => frus_text::measure(text, self.size).width,
            None => 0.0,
        }
    }

    /// The states it is in: the interaction's, ticked, disabled and in error.
    fn states(&self, status: &Status) -> WidgetStates {
        let states = if self.enabled {
            status.states()
        } else {
            WidgetStates::of(WidgetState::Disabled)
        };
        states
            .set(WidgetState::Selected, self.value != Some(false))
            .set(WidgetState::Error, self.error)
    }

    /// The fill for these states (`checkbox.dart:960`).
    fn fill(&self, theme: &Theme, states: WidgetStates) -> Color {
        let t = &theme.widgets.checkbox;
        if states.contains(WidgetState::Disabled) {
            return if states.contains(WidgetState::Selected) {
                disabled_content(theme)
            } else {
                Color::TRANSPARENT
            };
        }
        if !states.contains(WidgetState::Selected) {
            return Color::TRANSPARENT;
        }
        if states.contains(WidgetState::Error) {
            return theme.scheme.error;
        }
        self.fill_color
            .or(t.fill_color)
            .unwrap_or(theme.scheme.primary)
    }

    /// The mark's colour (`checkbox.dart:979`).
    fn check(&self, theme: &Theme, states: WidgetStates) -> Color {
        if states.contains(WidgetState::Disabled) {
            return disabled_mark(theme);
        }
        if states.contains(WidgetState::Error) {
            return theme.scheme.on_error;
        }
        self.check_color
            .or(theme.widgets.checkbox.check_color)
            .unwrap_or(theme.scheme.on_primary)
    }

    /// The outline of an unticked box for these states (`checkbox.dart:933`): none once
    /// ticked, faint disabled, the error colour in error, `on_surface` under a pointer, the
    /// keyboard or a finger, and `on_surface_variant` at rest.
    fn side(&self, theme: &Theme, states: WidgetStates) -> BorderSide {
        let t = &theme.widgets.checkbox;
        if states.contains(WidgetState::Selected) {
            return BorderSide::new(Color::TRANSPARENT, 0.0);
        }
        let color = if states.contains(WidgetState::Disabled) {
            disabled_content(theme)
        } else if states.contains(WidgetState::Error) {
            theme.scheme.error
        } else {
            let resting = self.border_color.or(t.border_color);
            let active = states.contains(WidgetState::Pressed)
                || states.contains(WidgetState::Hovered)
                || states.contains(WidgetState::Focused);
            if active {
                self.active_border_color
                    .or(t.active_border_color)
                    .or(resting)
                    .unwrap_or(theme.scheme.on_surface)
            } else {
                resting.unwrap_or(theme.scheme.on_surface_variant)
            }
        };
        BorderSide::new(color, STROKE)
    }

    /// The halo for these states (`checkbox.dart:998`): the caller's, the theme's, then the
    /// reference's — 8 % under a pointer and 10 % focused or pressed, in `on_surface` or
    /// `primary` depending on whether the box is ticked, and in `error` in error.
    fn overlay(&self, theme: &Theme, states: WidgetStates) -> Color {
        if let Some(c) = self
            .overlay_color
            .as_ref()
            .and_then(|p| p.resolve(states))
            .or_else(|| {
                theme
                    .widgets
                    .checkbox
                    .overlay_color
                    .as_ref()
                    .and_then(|p| p.resolve(states))
            })
        {
            return *c;
        }
        let c = &theme.scheme;
        let pressed = states.contains(WidgetState::Pressed);
        let hovered = states.contains(WidgetState::Hovered);
        let focused = states.contains(WidgetState::Focused);
        let pick = |press: Color, hover: Color, focus: Color| {
            if pressed {
                press.with_alpha(0.1)
            } else if hovered {
                hover.with_alpha(0.08)
            } else if focused {
                focus.with_alpha(0.1)
            } else {
                Color::TRANSPARENT
            }
        };
        if states.contains(WidgetState::Error) && (pressed || hovered || focused) {
            return pick(c.error, c.error, c.error);
        }
        if states.contains(WidgetState::Selected) {
            pick(c.on_surface, c.primary, c.primary)
        } else {
            pick(c.primary, c.on_surface, c.on_surface)
        }
    }
}

/// The box at `t` of its animation, from its origin: full size at either end, a stroke
/// smaller half way (`checkbox.dart:718`).
fn outer_at(origin: Point, t: f32) -> Rect {
    let inset = 1.0 - (t - 0.5).abs() * 2.0;
    let size = EDGE - inset * STROKE;
    Rect::new(origin.x + inset, origin.y + inset, size, size)
}

/// The tick at `t`: the short stroke drawn over the first half, the long one over the
/// second (`checkbox.dart:750`).
fn check_path(origin: Point, t: f32) -> Path {
    let at = |x: f32, y: f32| Point::new(origin.x + EDGE * x, origin.y + EDGE * y);
    let (start, mid, end) = (at(0.15, 0.45), at(0.4, 0.7), at(0.85, 0.25));
    let lerp =
        |a: Point, b: Point, k: f32| Point::new(a.x + (b.x - a.x) * k, a.y + (b.y - a.y) * k);
    if t < 0.5 {
        Path::new()
            .move_to(start)
            .line_to(lerp(start, mid, t * 2.0))
    } else {
        Path::new()
            .move_to(start)
            .line_to(mid)
            .line_to(lerp(mid, end, (t - 0.5) * 2.0))
    }
}

/// The dash at `t`, growing from the middle outwards (`checkbox.dart:773`).
fn dash_path(origin: Point, t: f32) -> Path {
    let y = origin.y + EDGE * 0.5;
    let half = EDGE * 0.3 * t;
    let mid = origin.x + EDGE * 0.5;
    Path::new()
        .move_to(Point::new(mid - half, y))
        .line_to(Point::new(mid + half, y))
}

impl<Msg> Widget<Msg> for Checkbox<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    /// **A square a finger can hit**, 48 px at the standard density, with the 18 px box in
    /// its middle (`checkbox.dart:516`), and the label after it.
    fn style_themed(&self, theme: &Theme) -> Style {
        let square = self.square(theme);
        let line = self.label_style().line_height();
        Style {
            width: Dimension::Length((square + self.label_width()).ceil()),
            height: Dimension::Length(line.max(square).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let square = self.square(theme);
        let centre = Point::new(bounds.x + square * 0.5, bounds.y + bounds.height * 0.5);
        let states = self.states(&status);

        // The halo: under a pointer and the keyboard it fades in, under a finger it grows
        // (`toggleable.dart`'s radial reaction).
        if self.enabled {
            let radius = self
                .splash_radius
                .or(theme.widgets.checkbox.splash_radius)
                .unwrap_or(SPLASH);
            let halo = |scene: &mut Scene, state: WidgetState, amount: f32, r: f32| {
                if amount <= 0.0 || r <= 0.0 {
                    return;
                }
                let c = self.overlay(theme, states.set(state, true));
                if c.a > 0.0 {
                    scene.draw_rect(
                        Rect::new(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r),
                        c.with_alpha(c.a * amount.clamp(0.0, 1.0)).fade(o),
                        r,
                        0.0,
                        Color::TRANSPARENT,
                    );
                }
            };
            halo(scene, WidgetState::Hovered, status.hover_progress, radius);
            halo(scene, WidgetState::Focused, status.focus_progress, radius);
            let press = status.press_progress.clamp(0.0, 1.0);
            halo(scene, WidgetState::Pressed, 1.0, radius * press);
        }

        // The box at `t` of its animation (`checkbox.dart:786`): 0 unticked, 1 filled.
        let t = status.value.clamp(0.0, 1.0);
        let origin = Point::new(centre.x - EDGE * 0.5, centre.y - EDGE * 0.5);
        let off = states.set(WidgetState::Selected, false);
        let on = states.set(WidgetState::Selected, true);
        let fill = if t >= 0.25 {
            self.fill(theme, on)
        } else {
            self.fill(theme, off).lerp(self.fill(theme, on), t * 4.0)
        };
        let outer = outer_at(origin, t);
        let corner = self.corner(theme);
        let side = if t <= 0.5 {
            let (a, b) = (self.side(theme, off), self.side(theme, on));
            BorderSide::new(a.color.lerp(b.color, t), a.width + (b.width - a.width) * t)
        } else {
            self.side(theme, on)
        };
        scene.draw_rect(outer, fill.fade(o), corner, side.width, side.color.fade(o));
        if t > 0.5 {
            let k = (t - 0.5) * 2.0;
            let path = match self.value {
                None => dash_path(origin, k),
                _ => check_path(origin, k),
            };
            scene.paint_path(
                &path,
                None,
                Some(Stroke::new(self.check(theme, on).fade(o), STROKE)),
            );
        }

        if let Some(label) = &self.label {
            let color = if self.enabled {
                self.label_color
                    .or(theme.widgets.checkbox.label_color)
                    .unwrap_or(theme.on_surface)
            } else {
                disabled_content(theme)
            };
            let style = self.label_style();
            scene.text(
                Point::new(
                    bounds.x + square,
                    bounds.y + (bounds.height - style.line_height()) * 0.5,
                ),
                label.clone(),
                &style,
                color.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        let next = self.next();
        if let Some(make) = self.on_change.as_ref() {
            return Some(make(next));
        }
        // `on_toggle` cannot say the third answer, so a tristate box wired only to it
        // reports the two it can: partly on reads as on, which is what a click on it
        // moves away from.
        self.on_toggle
            .as_ref()
            .map(|make| make(next.unwrap_or(true)))
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    /// Filled or not: ticking fills the box and draws the mark over a fifth of a second.
    fn anim_target(&self) -> Option<f32> {
        Some(if self.value == Some(false) { 0.0 } else { 1.0 })
    }

    fn anim_duration(&self) -> f32 {
        TOGGLE_SECONDS
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // Still ticked or not, still announced: a reader who cannot change the answer is
        // still owed it.
        let mut s = frus_core::SemanticsProperties::new(frus_core::Role::CheckBox)
            .maybe_toggled(self.value);
        s = if self.enabled {
            s.clickable()
        } else {
            s.disabled(true)
        };
        if let Some(label) = self.label.as_ref().or(self.semantic_label.as_ref()) {
            s = s.label(label.clone());
        }
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{PathVerb, Primitive};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Set(bool),
        Maybe(Option<bool>),
    }

    /// A settled status: the box's animation at its end for `value`.
    fn settled(value: bool) -> Status {
        Status {
            opacity: 1.0,
            value: if value { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }

    fn painted(c: &Checkbox<Msg>, status: Status, theme: &Theme) -> Vec<Primitive> {
        let side = c.square(theme);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            c,
            Rect::new(0.0, 0.0, side, side),
            status,
            theme,
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    /// The box: the last rectangle, with its fill, outline and corner.
    fn the_box(prims: &[Primitive]) -> (Rect, Color, f32, Color, f32) {
        prims
            .iter()
            .rev()
            .find_map(|p| match p {
                Primitive::Rect {
                    rect,
                    color,
                    border_width,
                    border_color,
                    radius,
                    ..
                } => Some((*rect, *color, *border_width, *border_color, radius.top_left)),
                _ => None,
            })
            .expect("a box")
    }

    fn mark(prims: &[Primitive]) -> Option<(Vec<Point>, Color)> {
        prims.iter().find_map(|p| match p {
            Primitive::Path {
                path,
                stroke: Some(s),
                ..
            } => Some((
                path.verbs()
                    .iter()
                    .filter_map(|v| match v {
                        PathVerb::MoveTo(p) | PathVerb::LineTo(p) => Some(*p),
                        _ => None,
                    })
                    .collect(),
                s.color,
            )),
            _ => None,
        })
    }

    /// **The reference's box**: 18 px with a 2 px corner, centred in a 48 px square at the
    /// standard density, which a checkbox keeps on every platform unless told.
    #[test]
    fn the_box_is_the_reference_s() {
        let theme = Theme::dark().with_platform(frus_core::TargetPlatform::Windows);
        let c = Checkbox::<Msg>::new(false);
        let style = Widget::<Msg>::style_themed(&c, &theme);
        assert_eq!(
            (style.width, style.height),
            (Dimension::Length(48.0), Dimension::Length(48.0)),
            "standard density even on a desktop"
        );
        let (rect, fill, border, outline, corner) = the_box(&painted(&c, settled(false), &theme));
        assert_eq!(rect, Rect::new(15.0, 15.0, 18.0, 18.0));
        assert_eq!((fill, border, corner), (Color::TRANSPARENT, 2.0, 2.0));
        assert_eq!(outline, theme.scheme.on_surface_variant);
        // Told to be compact, it is 40; shrink-wrapped, 40 too.
        let compact =
            c_style(Checkbox::<Msg>::new(false).visual_density(crate::VisualDensity::COMPACT));
        assert_eq!(compact, 40.0);
        let shrunk = c_style(Checkbox::<Msg>::new(false).tap_target(TapTarget::ShrinkWrap));
        assert_eq!(shrunk, 40.0);
    }

    fn c_style(c: Checkbox<Msg>) -> f32 {
        match Widget::<Msg>::style_themed(&c, &Theme::dark()).height {
            Dimension::Length(v) => v,
            _ => unreachable!(),
        }
    }

    /// **Ticked, the box fills and the tick is a drawn stroke** through the reference's
    /// three points (`checkbox.dart:755`); partly ticked, a dash across the middle.
    #[test]
    fn the_tick_is_drawn_through_the_reference_s_points() {
        let theme = Theme::dark();
        let prims = painted(&Checkbox::<Msg>::new(true), settled(true), &theme);
        let (rect, fill, border, _, _) = the_box(&prims);
        assert_eq!(
            (rect, fill, border),
            (Rect::new(15.0, 15.0, 18.0, 18.0), theme.scheme.primary, 0.0)
        );
        let (points, colour) = mark(&prims).expect("a tick");
        assert_eq!(colour, theme.scheme.on_primary);
        let at = |x: f32, y: f32| Point::new(15.0 + 18.0 * x, 15.0 + 18.0 * y);
        assert_eq!(points, vec![at(0.15, 0.45), at(0.4, 0.7), at(0.85, 0.25)]);
        let (dash, _) = mark(&painted(
            &Checkbox::<Msg>::maybe(None),
            settled(true),
            &theme,
        ))
        .unwrap();
        let near = |a: Point, b: Point| (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3;
        assert!(
            near(dash[0], at(0.2, 0.5)) && near(dash[1], at(0.8, 0.5)),
            "{dash:?}"
        );
    }

    /// **Ticking animates**: half way the box is a stroke smaller and the tick not yet
    /// drawn; at three quarters the short stroke is done and the long one half way.
    #[test]
    fn ticking_animates_the_box_and_the_tick() {
        let theme = Theme::dark();
        let c = Checkbox::<Msg>::new(true);
        assert_eq!(Widget::<Msg>::anim_target(&c), Some(1.0));
        assert_eq!(
            Widget::<Msg>::anim_target(&Checkbox::<Msg>::new(false)),
            Some(0.0)
        );
        // Partly ticked fills the box too: the dash is on a filled box, as the tick is.
        assert_eq!(
            Widget::<Msg>::anim_target(&Checkbox::<Msg>::maybe(None)),
            Some(1.0)
        );
        assert_eq!(Widget::<Msg>::anim_duration(&c), 0.2);
        let half = painted(
            &c,
            Status {
                value: 0.5,
                ..settled(true)
            },
            &theme,
        );
        assert_eq!(the_box(&half).0, Rect::new(16.0, 16.0, 16.0, 16.0));
        assert!(mark(&half).is_none(), "no tick yet");
        let three = painted(
            &c,
            Status {
                value: 0.75,
                ..settled(true)
            },
            &theme,
        );
        let (points, _) = mark(&three).unwrap();
        let at = |x: f32, y: f32| Point::new(15.0 + 18.0 * x, 15.0 + 18.0 * y);
        // The short stroke whole, and the long one not yet begun: a segment of no length at
        // the turn, as the reference draws it at that instant (`checkbox.dart:763`).
        assert_eq!(points, vec![at(0.15, 0.45), at(0.4, 0.7), at(0.4, 0.7)]);
    }

    /// **The halo is the reference's**: under a pointer `on_surface` at 8 % round an
    /// unticked box and `primary` at 8 % round a ticked one, 20 px in radius.
    #[test]
    fn the_halo_is_the_reference_s() {
        let theme = Theme::dark();
        let hovered = |value: bool| Status {
            hover_progress: 1.0,
            interaction: crate::interaction::Interaction::Hovered,
            ..settled(value)
        };
        let prims = painted(&Checkbox::<Msg>::new(false), hovered(false), &theme);
        let halo = prims.iter().find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if rect.width == 40.0 => Some(*color),
            _ => None,
        });
        assert_eq!(halo, Some(theme.scheme.on_surface.with_alpha(0.08)));
        let prims = painted(&Checkbox::<Msg>::new(true), hovered(true), &theme);
        let halo = prims.iter().find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if rect.width == 40.0 => Some(*color),
            _ => None,
        });
        assert_eq!(halo, Some(theme.scheme.primary.with_alpha(0.08)));
        // Under a pointer the outline darkens to `on_surface`.
        let (_, _, _, outline, _) = the_box(&painted(
            &Checkbox::<Msg>::new(false),
            hovered(false),
            &theme,
        ));
        assert_eq!(outline, theme.scheme.on_surface);
    }

    /// **In error** the outline, the fill and the tick take the error roles.
    #[test]
    fn an_error_takes_the_error_colours() {
        let theme = Theme::dark();
        let (_, _, _, outline, _) = the_box(&painted(
            &Checkbox::<Msg>::new(false).error(true),
            settled(false),
            &theme,
        ));
        assert_eq!(outline, theme.scheme.error);
        let prims = painted(
            &Checkbox::<Msg>::new(true).error(true),
            settled(true),
            &theme,
        );
        assert_eq!(the_box(&prims).1, theme.scheme.error);
        assert_eq!(mark(&prims).unwrap().1, theme.scheme.on_error);
    }

    /// **Disabled**: inert, but still showing whether it is ticked — the fill on_surface at
    /// 38 % and the tick in `surface`, resolved opaque.
    #[test]
    fn a_disabled_box_is_inert_but_still_says_whether_it_is_ticked() {
        let theme = Theme::dark();
        let c = Checkbox::<Msg>::new(true)
            .enabled(false)
            .on_toggle(Msg::Set);
        assert_eq!(Widget::<Msg>::on_click(&c), None);
        assert!(!Widget::<Msg>::focusable(&c));
        let prims = painted(&c, settled(true), &theme);
        assert_eq!(the_box(&prims).1, disabled_content(&theme));
        assert_eq!(mark(&prims).unwrap().1, disabled_mark(&theme));
    }

    #[test]
    fn click_toggles() {
        let c = Checkbox::<Msg>::new(false).on_toggle(Msg::Set);
        assert_eq!(Widget::<Msg>::on_click(&c), Some(Msg::Set(true)));
        let c = Checkbox::<Msg>::new(true).on_toggle(Msg::Set);
        assert_eq!(Widget::<Msg>::on_click(&c), Some(Msg::Set(false)));
    }

    #[test]
    fn a_tristate_box_cycles_through_the_third_answer() {
        let next = |v: Option<bool>| {
            Widget::<Msg>::on_click(&Checkbox::<Msg>::maybe(v).on_change(Msg::Maybe))
        };
        assert_eq!(next(Some(false)), Some(Msg::Maybe(Some(true))));
        assert_eq!(next(Some(true)), Some(Msg::Maybe(None)));
        assert_eq!(next(None), Some(Msg::Maybe(Some(false))));
        // On the two-state callback, partly on reads as on.
        let old = Checkbox::<Msg>::maybe(Some(true)).on_toggle(Msg::Set);
        assert_eq!(Widget::<Msg>::on_click(&old), Some(Msg::Set(true)));
    }

    #[test]
    fn partly_ticked_is_announced_as_mixed() {
        let s = Widget::<Msg>::semantics(&Checkbox::<Msg>::maybe(None)).unwrap();
        assert_eq!(s.toggled, frus_core::Toggled::Mixed);
        let named =
            Widget::<Msg>::semantics(&Checkbox::<Msg>::new(true).semantic_label("Agree")).unwrap();
        assert_eq!(named.label.as_deref(), Some("Agree"));
    }

    /// **The caller outranks the theme, which outranks the reference.**
    #[test]
    fn the_theme_answers_and_the_instance_overrules_it() {
        let mut theme = Theme::dark();
        let green = Color::rgb8(0, 160, 80);
        let blue = Color::rgb8(0, 0, 200);
        theme.widgets.checkbox.fill_color = Some(green);
        theme.widgets.checkbox.radius = Some(4.0);
        let themed = the_box(&painted(&Checkbox::<Msg>::new(true), settled(true), &theme));
        assert_eq!((themed.1, themed.4), (green, 4.0));
        let mine = the_box(&painted(
            &Checkbox::<Msg>::new(true).fill_color(blue).radius(1.0),
            settled(true),
            &theme,
        ));
        assert_eq!((mine.1, mine.4), (blue, 1.0));
        // One outline colour covers rest and pointer alike.
        let one = Checkbox::<Msg>::new(false).border_color(blue);
        let hovered = Status {
            interaction: crate::interaction::Interaction::Hovered,
            ..settled(false)
        };
        assert_eq!(the_box(&painted(&one, hovered, &theme)).3, blue);
    }

    /// **The label follows the box's square** and takes its colour.
    #[test]
    fn the_label_follows_the_square() {
        let theme = Theme::dark();
        let c = Checkbox::<Msg>::new(false)
            .label("Remember me")
            .label_color(Color::rgb8(9, 9, 9));
        let prims = painted(&c, settled(false), &theme);
        let at = prims.iter().find_map(|p| match p {
            Primitive::Text {
                position, color, ..
            } => Some((*position, *color)),
            _ => None,
        });
        let (position, colour) = at.expect("a label");
        assert_eq!(position.x, 48.0);
        assert_eq!(colour, Color::rgb8(9, 9, 9));
    }
}
