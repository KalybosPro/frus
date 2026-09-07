//! **The outline of a box**: [`ShapeBorder`] and [`BorderSide`] (`shape_border.dart`).
//!
//! Until this module a box here had a `radius: f32` and, at best, a [`BorderRadius`]. The
//! reference has a whole family, and it is not decoration: `shape` is the property that
//! decides a chip is a stadium, a floating button is a circle, and a navigation rail's
//! selection indicator is a pill rather than a rounded box. A framework with no way to
//! say *what shape this is* cannot accept the property at all, which is why
//! `indicatorShape`, `Card.shape`, `Chip.shape` and the rest have been recorded as
//! blocked since milestone 437.
//!
//! # Two ways out of one type
//!
//! Three of the four shapes here **are** rounded rectangles once the box is known — a
//! stadium is one whose radius is half the short side, a circle is one drawn in a square
//! taken out of the middle. [`ShapeBorder::as_rounded`] says so, and hands back the box
//! and the radii, so those three go down the renderer's existing fast path and cost
//! nothing new.
//!
//! [`ShapeBorder::outline`] answers for **all** of them, as a path. That is what a
//! bevelled corner needs, and what clipping to a shape will need when it arrives.

use crate::decoration::BorderRadius;
use crate::geometry::{Point, Rect};
use crate::path::Path;
use crate::Color;

/// **One edge of a shape**: what colour it is drawn in and how thick it is
/// (`border_side.dart`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BorderSide {
    pub color: Color,
    pub width: f32,
}

impl BorderSide {
    /// No edge at all — the reference's `BorderStyle.none`, said as a width rather than
    /// as a third state, since a zero-width edge and an absent one paint identically and
    /// nothing here has to tell them apart.
    pub const NONE: Self = Self {
        color: Color::TRANSPARENT,
        width: 0.0,
    };

    pub const fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }

    /// Is there an edge to draw?
    pub fn is_drawn(&self) -> bool {
        self.width > 0.0 && self.color.a > 0.0
    }
}

impl Default for BorderSide {
    fn default() -> Self {
        Self::NONE
    }
}

/// **What shape a box is** (`shape_border.dart`), and the edge around it.
///
/// `Copy`, deliberately: this is meant to sit in a theme, and the theme's per-widget
/// structs are copied wholesale. A shape with a variable number of corners — the
/// reference's `StarBorder`, `LinearBorder` — would end that, and is not here.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ShapeBorder {
    /// Corners rounded by a radius each (`rounded_rectangle_border.dart`).
    RoundedRectangle {
        radius: BorderRadius,
        side: BorderSide,
    },
    /// **A pill**: the ends are semicircles, whatever the box's proportions
    /// (`stadium_border.dart:95`). What a chip is, and what a navigation indicator is.
    Stadium { side: BorderSide },
    /// **A circle** inscribed in the box, or an ellipse on the way to filling it
    /// (`circle_border.dart:126`).
    Circle {
        /// `0.0` — a true circle, in a square taken out of the middle of the box. `1.0` —
        /// an ellipse filling the box entirely. Between them, the circle grows towards
        /// the ellipse along the box's longer axis.
        eccentricity: f32,
        side: BorderSide,
    },
    /// Corners **cut off straight** rather than rounded
    /// (`beveled_rectangle_border.dart`). The one shape here that is not a rounded
    /// rectangle in disguise.
    Beveled {
        radius: BorderRadius,
        side: BorderSide,
    },
}

impl Default for ShapeBorder {
    fn default() -> Self {
        Self::RoundedRectangle {
            radius: BorderRadius::ZERO,
            side: BorderSide::NONE,
        }
    }
}

impl ShapeBorder {
    /// A rectangle with these corners and no edge.
    pub fn rounded(radius: impl Into<BorderRadius>) -> Self {
        Self::RoundedRectangle {
            radius: radius.into(),
            side: BorderSide::NONE,
        }
    }

    /// A pill with no edge.
    pub const fn stadium() -> Self {
        Self::Stadium {
            side: BorderSide::NONE,
        }
    }

    /// A circle with no edge.
    pub const fn circle() -> Self {
        Self::Circle {
            eccentricity: 0.0,
            side: BorderSide::NONE,
        }
    }

    /// A rectangle with these corners cut off straight, and no edge.
    pub fn beveled(radius: impl Into<BorderRadius>) -> Self {
        Self::Beveled {
            radius: radius.into(),
            side: BorderSide::NONE,
        }
    }

    /// The same shape with an edge around it (the reference's `copyWith(side:)`).
    #[must_use]
    pub fn with_side(self, side: BorderSide) -> Self {
        match self {
            Self::RoundedRectangle { radius, .. } => Self::RoundedRectangle { radius, side },
            Self::Stadium { .. } => Self::Stadium { side },
            Self::Circle { eccentricity, .. } => Self::Circle { eccentricity, side },
            Self::Beveled { radius, .. } => Self::Beveled { radius, side },
        }
    }

    /// The edge around it.
    pub fn side(&self) -> BorderSide {
        match *self {
            Self::RoundedRectangle { side, .. }
            | Self::Stadium { side }
            | Self::Circle { side, .. }
            | Self::Beveled { side, .. } => side,
        }
    }

    /// **The rounded rectangle this shape is** inside `rect`: the box to draw and the
    /// radii to draw it with — or `None` when the outline is not one and wants
    /// [`outline`](Self::outline).
    ///
    /// A stadium's radius is half its short side (`stadium_border.dart:95`); a circle's
    /// box is the one `_adjustRect` takes out of the middle (`circle_border.dart:126`),
    /// with half *its* short side. Saying it this way is what keeps three shapes out of
    /// the path renderer.
    pub fn as_rounded(&self, rect: Rect) -> Option<(Rect, BorderRadius)> {
        match *self {
            Self::RoundedRectangle { radius, .. } => Some((rect, radius)),
            Self::Stadium { .. } => Some((rect, BorderRadius::uniform(shortest(rect) * 0.5))),
            Self::Circle { eccentricity, .. } => {
                let box_ = circle_box(rect, eccentricity);
                Some((box_, BorderRadius::uniform(shortest(box_) * 0.5)))
            }
            Self::Beveled { .. } => None,
        }
    }

    /// **The outline**, as a path — for every shape, including the three that did not
    /// need one.
    pub fn outline(&self, rect: Rect) -> Path {
        match *self {
            Self::Beveled { radius, .. } => beveled_path(rect, radius),
            _ => {
                let (box_, radius) = self
                    .as_rounded(rect)
                    .expect("every shape but a bevel is a rounded rectangle");
                rounded_path(box_, radius)
            }
        }
    }
}

/// The shorter of a box's two sides.
fn shortest(rect: Rect) -> f32 {
    rect.width.min(rect.height).max(0.0)
}

/// The box a circle of this eccentricity occupies inside `rect`
/// (`circle_border.dart:126`): a square out of the middle at `0.0`, the whole box at
/// `1.0`, and the difference shared equally between the two ends in between.
fn circle_box(rect: Rect, eccentricity: f32) -> Rect {
    let e = eccentricity.clamp(0.0, 1.0);
    if rect.width == rect.height {
        return rect;
    }
    if rect.width < rect.height {
        let delta = (1.0 - e) * (rect.height - rect.width) * 0.5;
        Rect::new(
            rect.x,
            rect.y + delta,
            rect.width,
            rect.height - delta * 2.0,
        )
    } else {
        let delta = (1.0 - e) * (rect.width - rect.height) * 0.5;
        Rect::new(
            rect.x + delta,
            rect.y,
            rect.width - delta * 2.0,
            rect.height,
        )
    }
}

/// Each corner's radius, never more than the box can give it.
fn fitted(rect: Rect, radius: BorderRadius) -> [f32; 4] {
    let cap = shortest(rect) * 0.5;
    [
        radius.top_left.clamp(0.0, cap),
        radius.top_right.clamp(0.0, cap),
        radius.bottom_right.clamp(0.0, cap),
        radius.bottom_left.clamp(0.0, cap),
    ]
}

/// A rounded rectangle as a path, its corners drawn as quarter arcs.
fn rounded_path(rect: Rect, radius: BorderRadius) -> Path {
    let [tl, tr, br, bl] = fitted(rect, radius);
    let (l, t) = (rect.x, rect.y);
    let (r, b) = (rect.x + rect.width, rect.y + rect.height);
    const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
    Path::new()
        .move_to(Point::new(l + tl, t))
        .line_to(Point::new(r - tr, t))
        .arc_to(Point::new(r - tr, t + tr), tr, -HALF_PI, 0.0)
        .line_to(Point::new(r, b - br))
        .arc_to(Point::new(r - br, b - br), br, 0.0, HALF_PI)
        .line_to(Point::new(l + bl, b))
        .arc_to(
            Point::new(l + bl, b - bl),
            bl,
            HALF_PI,
            std::f32::consts::PI,
        )
        .line_to(Point::new(l, t + tl))
        .arc_to(
            Point::new(l + tl, t + tl),
            tl,
            std::f32::consts::PI,
            std::f32::consts::PI + HALF_PI,
        )
        .close()
}

/// The same rectangle with its corners **cut off straight**.
fn beveled_path(rect: Rect, radius: BorderRadius) -> Path {
    let [tl, tr, br, bl] = fitted(rect, radius);
    let (l, t) = (rect.x, rect.y);
    let (r, b) = (rect.x + rect.width, rect.y + rect.height);
    Path::new()
        .move_to(Point::new(l + tl, t))
        .line_to(Point::new(r - tr, t))
        .line_to(Point::new(r, t + tr))
        .line_to(Point::new(r, b - br))
        .line_to(Point::new(r - br, b))
        .line_to(Point::new(l + bl, b))
        .line_to(Point::new(l, b - bl))
        .line_to(Point::new(l, t + tl))
        .close()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A stadium's radius is half its short side** (`stadium_border.dart:95`), which is
    /// what makes its ends semicircles whatever the box's proportions — and not a corner
    /// radius someone has to keep in step with the height.
    #[test]
    fn a_stadium_is_a_pill_at_any_size() {
        let wide = Rect::new(0.0, 0.0, 200.0, 32.0);
        let (box_, radius) = ShapeBorder::stadium().as_rounded(wide).unwrap();
        assert_eq!(box_, wide, "it fills the box it is given");
        assert_eq!(radius, BorderRadius::uniform(16.0));

        let tall = Rect::new(0.0, 0.0, 32.0, 200.0);
        let (_, radius) = ShapeBorder::stadium().as_rounded(tall).unwrap();
        assert_eq!(
            radius,
            BorderRadius::uniform(16.0),
            "the *short* side, either way"
        );
    }

    /// **A circle sits in a square taken out of the middle** of the box
    /// (`circle_border.dart:127`), and eccentricity walks it out to an ellipse filling
    /// the box.
    #[test]
    fn a_circle_takes_a_square_out_of_the_middle() {
        let wide = Rect::new(10.0, 0.0, 100.0, 40.0);
        let (box_, radius) = ShapeBorder::circle().as_rounded(wide).unwrap();
        assert_eq!(box_, Rect::new(40.0, 0.0, 40.0, 40.0), "centred and square");
        assert_eq!(radius, BorderRadius::uniform(20.0));

        let full = ShapeBorder::Circle {
            eccentricity: 1.0,
            side: BorderSide::NONE,
        };
        assert_eq!(
            full.as_rounded(wide).unwrap().0,
            wide,
            "an eccentricity of one fills the box"
        );

        let half = ShapeBorder::Circle {
            eccentricity: 0.5,
            side: BorderSide::NONE,
        };
        assert_eq!(
            half.as_rounded(wide).unwrap().0,
            Rect::new(25.0, 0.0, 70.0, 40.0),
            "and half way is half way"
        );
    }

    /// **A bevel is the one shape that is not a rounded rectangle in disguise**, so it is
    /// the one that has to go through the path renderer — eight straight segments where
    /// the rounded one has four arcs.
    #[test]
    fn a_bevel_is_the_one_that_needs_a_path() {
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        assert!(ShapeBorder::beveled(8.0).as_rounded(rect).is_none());
        assert!(ShapeBorder::rounded(8.0).as_rounded(rect).is_some());
        assert!(ShapeBorder::stadium().as_rounded(rect).is_some());

        // And every shape still answers with an outline.
        for shape in [
            ShapeBorder::beveled(8.0),
            ShapeBorder::rounded(8.0),
            ShapeBorder::stadium(),
            ShapeBorder::circle(),
        ] {
            assert!(
                !shape.outline(rect).verbs().is_empty(),
                "{shape:?} has no outline"
            );
        }
    }

    /// A corner never asks for more than the box can give it — the reference clamps too,
    /// and without it a radius larger than the box turns the outline inside out.
    #[test]
    fn a_corner_is_capped_by_the_box() {
        let rect = Rect::new(0.0, 0.0, 20.0, 10.0);
        assert_eq!(fitted(rect, BorderRadius::uniform(100.0)), [5.0; 4]);
        assert_eq!(fitted(rect, BorderRadius::uniform(-4.0)), [0.0; 4]);
    }

    /// An edge is a width and a colour, and `NONE` is the absence of one.
    #[test]
    fn an_edge_is_drawn_only_when_there_is_one() {
        assert!(!BorderSide::NONE.is_drawn());
        assert!(!BorderSide::new(Color::rgb8(255, 0, 0), 0.0).is_drawn());
        assert!(!BorderSide::new(Color::TRANSPARENT, 2.0).is_drawn());
        assert!(BorderSide::new(Color::rgb8(255, 0, 0), 1.0).is_drawn());
        assert_eq!(
            ShapeBorder::stadium()
                .with_side(BorderSide::new(Color::rgb8(255, 0, 0), 2.0))
                .side()
                .width,
            2.0
        );
    }
}
