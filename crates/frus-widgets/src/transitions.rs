//! The **explicit** transitions: a widget driven by a value the *application* owns.
//!
//! The other family, in [`crate::animated`], is implicit: you change a target and the
//! widget catches up on the framework's clock. These take the number itself — a progress
//! the caller stepped, a spring's current value, a fraction of a gesture — so their timing
//! is the application's, and two of them handed the same number move together.
//!
//! ## What they are, here
//!
//! In the reference this is a family of fifteen classes, because there a widget that
//! animates has to *listen*: an `Animation<double>` is an object, and each transition
//! subscribes to one and rebuilds when it ticks. There is no listening here and no retained
//! tree — the view is rebuilt every frame and an animation is a number — so each of these
//! is a **thin wrapper over a widget that already takes a value**, and that is not a
//! shortcut but the shape of the thing.
//!
//! What the names buy is that the rule is written once. `SlideTransition::from_edge` is a
//! sign and a subtraction that every application would otherwise get wrong once; the
//! progress conventions (`0` = not started, `1` = arrived) are stated in one place; and a
//! reader coming from the reference's vocabulary finds them under the names they know.
//!
//! ## The five here, and the ten that are not
//!
//! These three are what a **transition between two states of a screen** is made of, in the
//! reference too: a fade, a slide, and a scale. The rest of that file maps onto this
//! framework rather than being missing from it:
//!
//! - `RotationTransition` is [`crate::Transform::rotate`] on a value — or
//!   [`crate::AnimatedRotation`] when the timing is not the caller's.
//! - `PositionedTransition` and `RelativePositionedTransition` are [`crate::Positioned`]
//!   with interpolated pins; `AlignTransition` is `Container::alignment`;
//!   `MatrixTransition` is [`crate::Transform`]'s composed matrix.
//! - `AnimatedBuilder`, `ListenableBuilder` and `AnimatedWidget` are **nothing** here, and
//!   that is the whole difference between the two frameworks: they exist to rebuild a
//!   subtree when a value changes, and here the subtree is rebuilt every frame anyway.
//! - `SliverFadeTransition` waits on slivers, which this framework does not have.
//! - `SizeTransition` animates a size it has to **measure**, and a widget cannot measure
//!   its children here — see the entry in `ROADMAP.md` and issue #52. Not a wrapper, and
//!   not this milestone.
//! - [`DecoratedBoxTransition`] and [`DefaultTextStyleTransition`] are here since
//!   milestone 501. They are not wrappers over one value but interpolations of a whole
//!   decoration and a whole text style, each part by its own rule — see
//!   `BoxDecoration::lerp` and `TextStyle::lerp`, which is where the real work is.

use frus_core::{Alignment, BoxDecoration, Curve, Rect, Scene, TextAlign, TextOverflow, TextStyle};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// **Fades** its child by a value the caller owns: `0.0` invisible, `1.0` solid.
///
/// The subtree is composited as one group, so overlapping children do not darken where
/// they overlap — the difference between fading a picture and fading each of its parts.
///
/// ```
/// use frus_widgets::{FadeTransition, Text};
///
/// let entering = 0.4_f32;
/// let _fading: FadeTransition<()> = FadeTransition::new(entering, Text::new("Details"));
/// ```
///
/// It is [`crate::Opacity`] with the name the reference uses, which is worth having: a
/// reader looking for the explicit half of the animation library looks for this word.
pub struct FadeTransition<Msg> {
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> FadeTransition<Msg> {
    /// Fades `child` to `opacity`, clamped to `0..=1`.
    pub fn new(opacity: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(crate::Opacity::new(opacity.clamp(0.0, 1.0), child)),
        }
    }
}

impl<Msg> FadeTransition<Msg> {
    /// A transparent wrapper: the box is the child's.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(FadeTransition {
    /// Forwarded, all of them: a fade is not an identity, not a place, not a theme and
    /// not a surface. It is its child, disappearing.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

/// The edge a [`SlideTransition::from_edge`] comes in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlideFrom {
    /// From beyond the left edge, moving right.
    Left,
    /// From beyond the right edge, moving left.
    Right,
    /// From above, moving down.
    Top,
    /// From below, moving up.
    Bottom,
}

/// **Slides** its child by a value the caller owns, stated as a **fraction of the child's
/// own size**: `(1.0, 0.0)` is one whole width to the right.
///
/// A fraction rather than pixels because the width is the layout's to decide, and the
/// number that puts a panel exactly off its own edge is `1` whatever that width turns out
/// to be — which is what makes the same call work for a sheet, a drawer and a row.
///
/// Layout is untouched: the box stays where it was put and the neighbours do not move.
///
/// ```
/// use frus_widgets::{SlideFrom, SlideTransition, Text};
///
/// let entering = 0.25_f32;
/// // A quarter of the way in from the left: three quarters of a width still outside.
/// let _panel: SlideTransition<()> =
///     SlideTransition::from_edge(SlideFrom::Left, entering, Text::new("Filters"));
/// ```
///
/// [`SlideTransition::from_edge`] is the reason this has a name at all. *Coming in from
/// the left* is a negative offset that shrinks to nought as the progress grows, and every
/// application that writes it by hand gets the sign wrong once.
pub struct SlideTransition<Msg> {
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> SlideTransition<Msg> {
    /// Slides `child` by `fx` widths across and `fy` heights down.
    pub fn new(fx: f32, fy: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(crate::FractionalTranslation::new(fx, fy).child(child)),
        }
    }

    /// Slides `child` in from an edge: `progress` of `0` leaves it a whole size outside,
    /// `1` puts it exactly where the layout laid it out. Values between the two are the
    /// journey, and values outside them overshoot — which is what a spring does at the end
    /// of its travel, and is deliberately not clamped away.
    pub fn from_edge(edge: SlideFrom, progress: f32, child: impl Widget<Msg> + 'static) -> Self {
        let away = 1.0 - progress;
        let (fx, fy) = match edge {
            SlideFrom::Left => (-away, 0.0),
            SlideFrom::Right => (away, 0.0),
            SlideFrom::Top => (0.0, -away),
            SlideFrom::Bottom => (0.0, away),
        };
        Self::new(fx, fy, child)
    }
}

impl<Msg> SlideTransition<Msg> {
    /// A transparent wrapper: the box is the child's.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(SlideTransition {
    /// Forwarded, all of them: a slide is not an identity, not a place, not a theme and
    /// not a surface. It is its child, moving.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

/// **Scales** its child by a value the caller owns, about its centre or a pivot of the
/// caller's choosing. `1.0` is neutral.
///
/// Paint only: the box does not change, so nothing around it moves and the child may spill
/// past its own edges. That is what a control popping under a press wants, and what a
/// dialog growing into place wants.
///
/// ```
/// use frus_widgets::{ScaleTransition, Text};
///
/// let opening = 0.9_f32;
/// let _dialog: ScaleTransition<()> = ScaleTransition::new(opening, Text::new("Delete?"));
/// ```
///
/// The pivot is a choice of origin rather than a quantity, so it is an argument here and
/// never a thing to interpolate — the same rule [`crate::AnimatedScale`] states.
pub struct ScaleTransition<Msg> {
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> ScaleTransition<Msg> {
    /// Scales `child` by `scale`, about its centre.
    pub fn new(scale: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self::about(scale, Alignment::CENTER, child)
    }

    /// The same, about a `pivot` — the corner or edge that stays put.
    pub fn about(scale: f32, pivot: Alignment, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(
                crate::Transform::scale_xy_from(scale, scale, pivot)
                    // No duration: an explicit transition is **already** where the caller
                    // says it is. A tween here would chase the value the application is
                    // itself stepping, and the two clocks would fight.
                    .child(child),
            ),
        }
    }
}

impl<Msg> ScaleTransition<Msg> {
    /// A transparent wrapper: the box is the child's.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(ScaleTransition {
    /// Forwarded, all of them: a scale is not an identity, not a place, not a theme and
    /// not a surface. It is its child, growing.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

/// A curve applied to a progress, for a caller stepping one by hand: `curve.transform(t)`
/// is the same call the framework's own animations make, so an explicit transition and an
/// implicit one given the same curve are on the same shape.
///
/// It is here rather than left to the caller because reaching for
/// [`frus_core::Curve::transform`] means knowing it exists, and the name of the thing you
/// are looking for is *ease this progress*.
pub fn eased(progress: f32, curve: &Curve) -> f32 {
    curve.transform(progress.clamp(0.0, 1.0))
}

/// Where a [`DecoratedBoxTransition`] paints its decoration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecorationPosition {
    /// Behind the child: a fill, a card, a highlight under a row. The default.
    #[default]
    Background,
    /// Over the child: a border around a photograph, a wash across a tile that is
    /// becoming disabled.
    Foreground,
}

/// **Paints a decoration that moves** by a value the caller owns — its fill, gradient,
/// border, corners and shadow, each travelling by its own rule (see
/// [`BoxDecoration::lerp`]).
///
/// ```
/// use frus_core::{Border, BoxDecoration, Color};
/// use frus_widgets::{DecoratedBoxTransition, Text};
///
/// let selected = 0.6_f32; // the application's own progress
/// let _row: DecoratedBoxTransition<()> = DecoratedBoxTransition::between(
///     BoxDecoration::default().radius(4.0),
///     BoxDecoration::filled(Color::rgb(0.2, 0.4, 0.9))
///         .radius(12.0)
///         .border(Border::new(2.0, Color::rgb(0.1, 0.2, 0.6))),
///     selected,
///     Text::new("Reminders"),
/// );
/// ```
///
/// **Unlike the other three here, this is a node of its own**, with the child beneath it:
/// a decoration is something painted, and something painted has a box. That box is the
/// child's, and **the child is not inset by the border** — the same as the reference's
/// `DecoratedBox`, and for a reason that matters more here than there: a line that
/// thickens over a transition would otherwise push the content about on every frame of
/// it. A border that must not overlap the content wants padding the caller chooses.
///
/// For a decoration that should catch up with a target on the framework's clock instead,
/// [`crate::AnimatedContainer`] is the implicit half.
pub struct DecoratedBoxTransition<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    decoration: BoxDecoration,
    position: DecorationPosition,
}

impl<Msg> DecoratedBoxTransition<Msg> {
    /// `child` over `decoration`, as it stands.
    pub fn new(decoration: BoxDecoration, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            children: vec![Box::new(child)],
            decoration,
            position: DecorationPosition::Background,
        }
    }

    /// `child` over the decoration `progress` of the way from `from` to `to` — which is
    /// what this is nearly always called with.
    pub fn between(
        from: BoxDecoration,
        to: BoxDecoration,
        progress: f32,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self::new(from.lerp(to, progress), child)
    }

    /// Paints the decoration over the child instead of behind it.
    pub fn position(mut self, position: DecorationPosition) -> Self {
        self.position = position;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for DecoratedBoxTransition<Msg> {
    fn style(&self) -> Style {
        // Nothing of its own: the box is the child's, and the border does not inset it.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, _theme: &Theme, scene: &mut Scene) {
        if self.position == DecorationPosition::Background {
            self.decoration.paint_into(scene, bounds, status.opacity);
        }
    }

    /// In front of the child, through the walk's one place for painting after a subtree.
    fn foreground(&self, _theme: &Theme) -> Option<BoxDecoration> {
        (self.position == DecorationPosition::Foreground).then_some(self.decoration)
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "DecoratedBoxTransition"
    }
}

/// **Hands a text style that moves down to everything below it**, by a value the caller
/// owns. The explicit half of [`crate::AnimatedDefaultTextStyle`].
///
/// ```
/// use frus_core::TextStyle;
/// use frus_widgets::{DefaultTextStyleTransition, Text};
///
/// let arriving = 0.3_f32; // the application's own progress
/// let _title: DefaultTextStyleTransition<()> = DefaultTextStyleTransition::between(
///     TextStyle::NONE.size(20.0),
///     TextStyle::NONE.size(24.0),
///     arriving,
///     Text::new("Groceries"),
/// );
/// ```
///
/// What travels, what holds and what swaps half-way is [`TextStyle::lerp`]'s business.
/// Alignment, wrapping, overflow and the line count take effect at once, as they do on the
/// implicit one: they are arrangements, not quantities.
///
/// **It is a transparent wrapper, which its implicit twin cannot be.** An implicit
/// animation keeps a timeline in the runtime, and a timeline belongs to a node, so two of
/// those nested have to be two nodes or they would share one. This keeps nothing — the
/// caller's number is the whole of its state — so two nested fuse into one node and simply
/// hand down both of what they say, the inner one asked by the outer as a nested theme is.
///
/// A text that names a field for itself keeps it: an inherited style answers a question,
/// it does not overrule an answer.
pub struct DefaultTextStyleTransition<Msg> {
    handed: crate::widgettheme::DefaultTextStyle,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg: Clone + 'static> DefaultTextStyleTransition<Msg> {
    /// `child`'s subtree wearing `style`, as it stands.
    pub fn new(style: TextStyle, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            handed: crate::widgettheme::DefaultTextStyle::from_text_style(style),
            inner: Box::new(child),
        }
    }

    /// `child`'s subtree wearing the style `progress` of the way from `from` to `to`.
    pub fn between(
        from: TextStyle,
        to: TextStyle,
        progress: f32,
        child: impl Widget<Msg> + 'static,
    ) -> Self {
        Self::new(from.lerp(to, progress), child)
    }

    /// Where the lines sit inside their box. **Not animated.**
    pub fn align(mut self, align: TextAlign) -> Self {
        self.handed.align = Some(align);
        self
    }

    /// Whether the text wraps at the width it is given. **Not animated.**
    pub fn soft_wrap(mut self, wrap: bool) -> Self {
        self.handed.soft_wrap = Some(wrap);
        self
    }

    /// What becomes of text that does not fit. **Not animated.**
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.handed.overflow = Some(overflow);
        self
    }

    /// At most this many lines. **Not animated.**
    pub fn max_lines(mut self, lines: usize) -> Self {
        self.handed.max_lines = Some(lines);
        self
    }
}

impl<Msg> DefaultTextStyleTransition<Msg> {
    /// A transparent wrapper: the box is the child's. A style reaches sizes through the
    /// child's own `style_themed`, not through this.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(DefaultTextStyleTransition {
    /// A style is not an identity, not a place and not a surface: forwarded.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }

    /// The one thing this does not delegate: the style it hands down, merged onto what it
    /// inherits — and then the child asked in turn, because a transparent wrapper *is* its
    /// child and a nested one would otherwise never be asked at all.
    fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
        let mut mine = inherited.clone();
        mine.widgets.text = mine.widgets.text.merge(self.handed);
        Some(
            self.inner
                .theme_override(&mine)
                .unwrap_or_else(|| Box::new(mine)),
        )
    }

    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Theme};
    use frus_core::{Color, Primitive, Rect, Size};

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    fn mark() -> Container<()> {
        Container::new().width(40.0).height(20.0).color(RED)
    }

    /// Where the blue neighbour was painted, through any layer above it.
    fn blue_y(primitives: &[Primitive]) -> Option<f32> {
        for primitive in primitives {
            match primitive {
                Primitive::Rect { rect, color, .. } if color.b > 0.5 && color.r < 0.5 => {
                    return Some(rect.y)
                }
                Primitive::Layer { primitives, .. } => {
                    if let Some(found) = blue_y(primitives) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The first red rectangle, and the opacity of the layer it is inside (`1.0` when
    /// there is no layer).
    fn painted(root: &dyn Widget<()>) -> (Rect, f32) {
        let ui = build_ui(
            root,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::dark(),
        );
        fn walk(primitives: &[Primitive], opacity: f32) -> Option<(Rect, f32)> {
            for primitive in primitives {
                match primitive {
                    Primitive::Rect { rect, color, .. } if color.r > 0.5 && color.g < 0.5 => {
                        return Some((*rect, opacity))
                    }
                    Primitive::Layer {
                        primitives,
                        opacity: layer,
                        ..
                    } => {
                        if let Some(found) = walk(primitives, opacity * layer) {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        walk(ui.scene().primitives(), 1.0).expect("the mark")
    }

    /// **Nothing, half way, all of it** — the three each of these has to pin.
    #[test]
    fn a_fade_is_the_value_it_was_given() {
        for (given, expected) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
            let root = FadeTransition::new(given, mark());
            let (_, opacity) = painted(&root);
            assert!(
                (opacity - expected).abs() < 1e-3,
                "at {given}: painted at {opacity}"
            );
        }
    }

    /// A slide is a multiple of the child's **own** box: a 40 px mark slid by half a width
    /// has moved 20.
    #[test]
    fn a_slide_is_a_multiple_of_the_child_s_own_box() {
        for (given, expected) in [(0.0, 0.0), (0.5, 20.0), (1.0, 40.0)] {
            let root = crate::Flex::<()>::column().child(SlideTransition::new(given, 0.0, mark()));
            let (rect, _) = painted(&root);
            assert!(
                (rect.x - expected).abs() < 0.5,
                "at {given}: painted at {}",
                rect.x
            );
        }
    }

    /// And the sign the name exists for: coming in from the left, nought is a whole width
    /// outside and one is home.
    #[test]
    fn coming_in_from_an_edge_arrives_at_one() {
        let at = |progress: f32| {
            let root = crate::Flex::<()>::column().child(SlideTransition::from_edge(
                SlideFrom::Left,
                progress,
                mark(),
            ));
            painted(&root).0.x
        };
        assert!((at(0.0) + 40.0).abs() < 0.5, "a whole width outside");
        assert!((at(0.5) + 20.0).abs() < 0.5, "half of one, still outside");
        assert!(at(1.0).abs() < 0.5, "and home");

        // Each edge sends it the way its name says.
        let mark_at = |edge: SlideFrom| {
            let root =
                crate::Flex::<()>::column().child(SlideTransition::from_edge(edge, 0.0, mark()));
            let rect = painted(&root).0;
            (rect.x, rect.y)
        };
        assert!(mark_at(SlideFrom::Left).0 < -1.0);
        assert!(mark_at(SlideFrom::Right).0 > 1.0);
        assert!(mark_at(SlideFrom::Top).1 < -1.0);
        assert!(mark_at(SlideFrom::Bottom).1 > 1.0);
    }

    /// A scale is a **paint** transform: it reaches the screen as a matrix on a composited
    /// layer, and the box underneath is untouched — so the neighbour below does not move
    /// however big the child is drawn. A neutral scale costs no layer at all.
    #[test]
    fn a_scale_is_paint_and_not_layout() {
        let built = |scale: f32| {
            let root = crate::Flex::<()>::column()
                .width(100.0)
                .child(ScaleTransition::new(scale, mark()))
                .child(Container::<()>::new().width(10.0).height(10.0).color(BLUE));
            let ui = build_ui(
                &root,
                Size::new(100.0, 100.0),
                &Runtime::default(),
                &Theme::dark(),
            );
            let matrix = ui.scene().primitives().iter().find_map(|p| match p {
                Primitive::Layer {
                    transform: Some(t), ..
                } => Some(t.affine.m[0]),
                _ => None,
            });
            let neighbour = blue_y(ui.scene().primitives()).expect("the neighbour");
            (matrix, neighbour)
        };
        for (given, expected) in [(0.5, Some(0.5)), (1.0, None), (2.0, Some(2.0))] {
            let (matrix, neighbour) = built(given);
            match (matrix, expected) {
                (Some(m), Some(e)) => {
                    assert!((m - e).abs() < 1e-3, "at {given}: painted at {m}")
                }
                (None, None) => {}
                _ => panic!("at {given}: {matrix:?} where {expected:?} was expected"),
            }
            assert!(
                (neighbour - 20.0).abs() < 0.5,
                "the neighbour stayed where the layout put it: {neighbour}"
            );
        }
    }

    /// The curve is the framework's own, so the two families agree on what *ease-out, half
    /// way* means.
    #[test]
    fn easing_a_progress_is_the_frameworks_own_curve() {
        let curve = Curve::ease_out();
        assert!(eased(0.0, &curve).abs() < 1e-3);
        assert!((eased(1.0, &curve) - 1.0).abs() < 1e-3);
        assert_eq!(eased(0.5, &curve), curve.transform(0.5));
        assert!(
            eased(0.5, &curve) > 0.5,
            "an ease-out is ahead at the middle"
        );
        assert_eq!(
            eased(2.0, &curve),
            eased(1.0, &curve),
            "and a progress past its end is its end"
        );
    }

    // --- A decoration and a text style (milestone 501) ---

    use frus_core::{Border, TextStyle};

    /// Every rectangle painted, in paint order, with its colour — through any layer.
    fn rects_of(root: &dyn Widget<()>) -> Vec<(Rect, Color)> {
        let ui = build_ui(
            root,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::dark(),
        );
        fn walk(primitives: &[Primitive], out: &mut Vec<(Rect, Color)>) {
            for primitive in primitives {
                match primitive {
                    Primitive::Rect { rect, color, .. } => out.push((*rect, *color)),
                    Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(ui.scene().primitives(), &mut out);
        out
    }

    /// Every run of words painted: its size and its colour.
    fn texts_of(root: &dyn Widget<()>) -> Vec<(f32, Color)> {
        let ui = build_ui(
            root,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::dark(),
        );
        fn walk(primitives: &[Primitive], out: &mut Vec<(f32, Color)>) {
            for primitive in primitives {
                match primitive {
                    Primitive::Text { size, color, .. } => out.push((*size, *color)),
                    Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(ui.scene().primitives(), &mut out);
        out
    }

    fn close(a: Color, b: Color) -> bool {
        (a.r - b.r).abs() < 1e-4
            && (a.g - b.g).abs() < 1e-4
            && (a.b - b.b).abs() < 1e-4
            && (a.a - b.a).abs() < 1e-4
    }

    /// **At the start, half-way and at the end — read off the scene**, which is where a
    /// decoration that moved correctly and never reached the screen would be caught.
    #[test]
    fn a_decoration_is_painted_as_it_stands() {
        for (t, want) in [(0.0, RED), (0.5, RED.lerp(BLUE, 0.5)), (1.0, BLUE)] {
            let tree = DecoratedBoxTransition::<()>::between(
                BoxDecoration::filled(RED),
                BoxDecoration::filled(BLUE),
                t,
                Container::new().width(40.0).height(20.0),
            );
            let rects = rects_of(&tree);
            assert_eq!(rects.len(), 1, "at {t}: {rects:?}");
            assert!(close(rects[0].1, want), "at {t}: {:?}", rects[0].1);
        }
    }

    /// **A fill fading out keeps its hue on the screen too**, not only in the arithmetic.
    #[test]
    fn a_fading_fill_is_painted_in_its_own_colour() {
        let tree = DecoratedBoxTransition::<()>::between(
            BoxDecoration::filled(RED),
            BoxDecoration::default(),
            0.5,
            Container::new().width(40.0).height(20.0),
        );
        let rects = rects_of(&tree);
        assert_eq!(rects.len(), 1, "{rects:?}");
        assert!(close(rects[0].1, RED.fade(0.5)), "{:?}", rects[0].1);
    }

    /// **A line that thickens does not move the content.** The child is not inset by the
    /// border, so a border growing from nothing to eight pixels leaves it where it was on
    /// every frame — which an inset would not.
    #[test]
    fn a_thickening_border_does_not_move_the_child() {
        let child_at = |t: f32| {
            let tree = DecoratedBoxTransition::<()>::between(
                BoxDecoration::default(),
                BoxDecoration::default().border(Border::new(8.0, BLUE)),
                t,
                mark(),
            );
            rects_of(&tree)
                .into_iter()
                .find(|(_, c)| close(*c, RED))
                .expect("the child")
                .0
        };
        let (start, end) = (child_at(0.0), child_at(1.0));
        assert_eq!(
            (start.x, start.y),
            (end.x, end.y),
            "the child stayed put: {start:?} then {end:?}"
        );
    }

    /// **In front, it is painted over the child**; behind is the default.
    #[test]
    fn in_front_the_decoration_covers_the_child() {
        let wash = || BoxDecoration::filled(BLUE.fade(0.3));
        let order = |tree: &DecoratedBoxTransition<()>| {
            let rects = rects_of(tree);
            let child = rects.iter().position(|(_, c)| close(*c, RED));
            let over = rects.iter().position(|(_, c)| c.b > 0.5);
            (child.expect("the child"), over.expect("the wash"))
        };
        let (child, over) = order(
            &DecoratedBoxTransition::new(wash(), mark()).position(DecorationPosition::Foreground),
        );
        assert!(child < over, "in front: the wash comes after the child");
        let (child, under) = order(&DecoratedBoxTransition::new(wash(), mark()));
        assert!(under < child, "behind, by default");
    }

    /// **The words are drawn at the style in flight** — start, half-way and end.
    #[test]
    fn a_text_style_is_handed_down_as_it_stands() {
        for (t, want) in [(0.0, 12.0), (0.5, 18.0), (1.0, 24.0)] {
            let tree = DefaultTextStyleTransition::<()>::between(
                TextStyle::NONE.size(12.0),
                TextStyle::NONE.size(24.0),
                t,
                crate::Text::new("Ag"),
            );
            let sizes: Vec<f32> = texts_of(&tree).into_iter().map(|(s, _)| s).collect();
            assert_eq!(sizes, vec![want], "at {t}");
        }
    }

    /// **Two nested fuse into one node and hand down both.** An outer one moving the
    /// colour and an inner one the size: the words wear both, which means the outer asked
    /// the inner — a transparent wrapper *is* its child, and the inner is never visited as a
    /// node of its own.
    #[test]
    fn two_nested_hand_down_both_halves() {
        let tree = DefaultTextStyleTransition::<()>::new(
            TextStyle::NONE.color(BLUE),
            DefaultTextStyleTransition::new(TextStyle::NONE.size(20.0), crate::Text::new("Ag")),
        );
        let texts = texts_of(&tree);
        assert_eq!(texts.len(), 1, "{texts:?}");
        assert_eq!(texts[0].0, 20.0, "the inner one's size");
        assert!(close(texts[0].1, BLUE), "and the outer one's colour");
    }

    /// A text that names its own size keeps it: an inherited style answers a question, it
    /// does not overrule an answer.
    #[test]
    fn a_text_that_names_its_size_keeps_it() {
        let tree = DefaultTextStyleTransition::<()>::new(
            TextStyle::NONE.size(30.0),
            crate::Text::new("Ag").size(11.0),
        );
        let sizes: Vec<f32> = texts_of(&tree).into_iter().map(|(s, _)| s).collect();
        assert_eq!(sizes, vec![11.0]);
    }
}
