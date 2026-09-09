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
//! ## The three here, and the twelve that are not
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
//! - `DecoratedBoxTransition` and `DefaultTextStyleTransition` interpolate a whole
//!   decoration and a whole text style. Both are real work — a decoration's colours,
//!   corners, border and shadow, each with its own rule — and a colour lerped in the wrong
//!   space is this repository's most repeated bug. They deserve their own milestone rather
//!   than a corner of this one.

use frus_core::{Alignment, Curve};
use frus_layout::Style;

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
}
