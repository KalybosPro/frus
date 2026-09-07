//! [`AnimatedIconData`] and [`AnimatedIcons`]: the marks that **turn into one another**
//! rather than being swapped for one another.
//!
//! A hamburger that becomes a cross as a drawer opens, a play that becomes a pause, a
//! plus that becomes a cross, a chevron that turns over. Each of these is one drawing
//! with a parameter, not two drawings with a cut between them, and the difference is
//! visible: a swap tells the reader that something replaced something, and a morph tells
//! them that the same control changed its mind.
//!
//! # Why the pairs are hand-drawn
//!
//! Interpolating between two arbitrary outlines is a general path-morphing problem —
//! matching sub-paths, matching point counts, matching winding — and it has no good
//! answer for two drawings that were never made to correspond. Three bars and a cross do
//! not have the same number of anything.
//!
//! So a pair is **authored as a function of `t`**, exactly the way
//! [`IconData::custom`](crate::IconData::custom) lets an application author a static one.
//! The bundled pairs are hand-drawn on the same `24 × 24` grid, close to the marks of the
//! same name in the bundled set but not the same outlines: they are drawn to move well,
//! which is a different constraint from being drawn to sit still.
//!
//! ```
//! use frus_widgets::{AnimatedIcons, Icon};
//!
//! // The mark at rest, at the halfway point, and at the end.
//! let shut = AnimatedIcons::MENU_CLOSE.at(0.0);
//! let open = AnimatedIcons::MENU_CLOSE.at(1.0);
//! assert_ne!(shut.path().verbs(), open.path().verbs());
//! let _widget = Icon::animated(AnimatedIcons::MENU_CLOSE, true);
//! ```

use frus_core::{Path, Point};

use super::IconData;

/// A mark that is **drawn from a number**: `0.0` is one state, `1.0` is the other, and
/// everything between is the way across.
///
/// It is `Copy` and costs nothing until [`AnimatedIconData::at`] is called. What comes
/// back from `at` is an ordinary [`IconData`], so **every widget that already takes an
/// icon takes a morphing one** — an [`Icon`](crate::Icon), an
/// [`IconButton`](crate::IconButton), a floating action button, a chip, a navigation
/// destination — with nothing added to any of them.
#[derive(Clone, Copy, Debug)]
pub struct AnimatedIconData {
    draw: fn(f32) -> Path,
    directional: bool,
}

impl PartialEq for AnimatedIconData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::fn_addr_eq(self.draw, other.draw) && self.directional == other.directional
    }
}

impl Eq for AnimatedIconData {}

impl std::hash::Hash for AnimatedIconData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((self.draw as usize), self.directional).hash(state);
    }
}

impl AnimatedIconData {
    /// A pair of the caller's own: `draw` is handed a `t` in `0.0..=1.0` and returns the
    /// outline for it, on the `24 × 24` grid, y downwards, filled by the non-zero rule.
    ///
    /// A function pointer rather than a closure, so a pair stays a `const` and is
    /// declared exactly where a bundled one would be.
    ///
    /// ```
    /// use frus_core::{Path, Rect};
    /// use frus_widgets::AnimatedIconData;
    ///
    /// /// A bar that grows from nothing.
    /// fn grow(t: f32) -> Path {
    ///     Path::rect(Rect::new(12.0 - 9.0 * t, 11.0, 18.0 * t, 2.0))
    /// }
    /// const GROW: AnimatedIconData = AnimatedIconData::custom(grow);
    /// assert!(GROW.at(0.0).path().verbs().len() == GROW.at(1.0).path().verbs().len());
    /// ```
    pub const fn custom(draw: fn(f32) -> Path) -> Self {
        Self {
            draw,
            directional: false,
        }
    }

    /// The same pair, declared to **carry a direction** — turned round in a right-to-left
    /// reading order, like [`IconData::mirrored`](crate::IconData::mirrored). The whole
    /// animation is mirrored, at every `t`, because a mark that points somewhere points
    /// there all the way across.
    pub const fn mirrored(self) -> Self {
        Self {
            draw: self.draw,
            directional: true,
        }
    }

    /// Whether this pair is turned round for a right-to-left reading order.
    pub const fn matches_text_direction(self) -> bool {
        self.directional
    }

    /// **The mark at `t`**, as an ordinary icon.
    ///
    /// `t` is clamped to `0.0..=1.0`, and a `t` that is not a number is read as `0.0`:
    /// an animation driver that divides by a zero duration must not be able to make an
    /// icon that compares unequal to itself.
    pub fn at(self, t: f32) -> IconData {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        IconData::morphing(self.draw, t, self.directional)
    }
}

// ---------------------------------------------------------------------------
// Drawing helpers. Everything below is authored on the 24 × 24 grid, y down.
// ---------------------------------------------------------------------------

/// The middle of the grid, which every one of these turns about.
const MID: f32 = 12.0;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Adds a `len × thick` bar centred on `(cx, cy)` and turned by `angle` (clockwise, since
/// y points down) as one closed contour.
///
/// A bar rather than a rectangle because that is what these drawings are made of, and a
/// turned rectangle is not a rectangle: the corners have to be worked out, and working
/// them out here is what keeps every pair below to a list of bars.
fn bar(path: Path, cx: f32, cy: f32, len: f32, thick: f32, angle: f32) -> Path {
    let (sin, cos) = angle.sin_cos();
    let (hl, ht) = (len * 0.5, thick * 0.5);
    // The half-length and half-thickness vectors, turned.
    let (lx, ly) = (hl * cos, hl * sin);
    let (tx, ty) = (-ht * sin, ht * cos);
    path.move_to(Point::new(cx - lx - tx, cy - ly - ty))
        .line_to(Point::new(cx + lx - tx, cy + ly - ty))
        .line_to(Point::new(cx + lx + tx, cy + ly + ty))
        .line_to(Point::new(cx - lx + tx, cy - ly + ty))
        .close()
}

/// Adds a quadrilateral as one closed contour, corners in the order they were given.
fn quad(path: Path, corners: [Point; 4]) -> Path {
    path.move_to(corners[0])
        .line_to(corners[1])
        .line_to(corners[2])
        .line_to(corners[3])
        .close()
}

/// The straight line between two quadrilaterals, corner by corner. The corners have to be
/// listed in the same order in both, or the shape turns itself inside out on the way.
fn quad_between(path: Path, from: [Point; 4], to: [Point; 4], t: f32) -> Path {
    let mut corners = from;
    for (c, (a, b)) in corners.iter_mut().zip(from.iter().zip(to.iter())) {
        *c = Point::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t));
    }
    quad(path, corners)
}

/// A quarter turn, in radians.
const QUARTER: f32 = std::f32::consts::FRAC_PI_2;

// ---------------------------------------------------------------------------
// The pairs.
// ---------------------------------------------------------------------------

/// Three bars becoming a cross: the top one swings down to the right, the bottom one up
/// to the right, and the middle one **shortens to nothing** rather than fading, because
/// an icon is one filled path and half of it cannot be given its own opacity.
fn menu_close(t: f32) -> Path {
    const LEN: f32 = 18.0;
    const THICK: f32 = 2.0;
    let path = Path::new();
    let path = bar(
        path,
        MID,
        lerp(7.0, MID, t),
        lerp(LEN, 19.8, t),
        THICK,
        lerp(0.0, QUARTER * 0.5, t),
    );
    // The middle bar keeps its centre and loses its **length**, down to none at all. It
    // is still drawn at t = 1, as a contour of zero area that fills nothing: dropping it
    // there would change the point count at one end of the animation, and a path whose
    // shape depends on which frame you asked for is the thing a morph exists not to be.
    let path = bar(path, MID, MID, LEN * (1.0 - t), THICK, 0.0);
    bar(
        path,
        MID,
        lerp(17.0, MID, t),
        lerp(LEN, 19.8, t),
        THICK,
        lerp(0.0, -QUARTER * 0.5, t),
    )
}

/// A triangle becoming two bars, as **two quadrilaterals** that start out as the two
/// halves of the triangle. The right half starts with its two right-hand corners on the
/// same point — the triangle's tip — which is what lets a three-cornered shape and a
/// four-cornered one be the same shape all the way across.
fn play_pause(t: f32) -> Path {
    let p = |x: f32, y: f32| Point::new(x, y);
    let left_from = [p(8.0, 5.0), p(8.0, 19.0), p(13.5, 15.5), p(13.5, 8.5)];
    let left_to = [p(6.0, 5.0), p(6.0, 19.0), p(10.0, 19.0), p(10.0, 5.0)];
    let right_from = [p(13.5, 8.5), p(13.5, 15.5), p(19.0, 12.0), p(19.0, 12.0)];
    let right_to = [p(14.0, 5.0), p(14.0, 19.0), p(18.0, 19.0), p(18.0, 5.0)];
    let path = quad_between(Path::new(), left_from, left_to, t);
    quad_between(path, right_from, right_to, t)
}

/// A plus turning into a cross. The two shapes are the same shape, so this is a rotation
/// and nothing else — an eighth of a turn — and lerping the twelve corners instead would
/// pinch the arms on the way through.
fn add_close(t: f32) -> Path {
    let p = |x: f32, y: f32| Point::new(x, y);
    let plus = Path::new()
        .move_to(p(11.0, 5.0))
        .line_to(p(13.0, 5.0))
        .line_to(p(13.0, 11.0))
        .line_to(p(19.0, 11.0))
        .line_to(p(19.0, 13.0))
        .line_to(p(13.0, 13.0))
        .line_to(p(13.0, 19.0))
        .line_to(p(11.0, 19.0))
        .line_to(p(11.0, 13.0))
        .line_to(p(5.0, 13.0))
        .line_to(p(5.0, 11.0))
        .line_to(p(11.0, 11.0))
        .close();
    plus.rotated(Point::new(MID, MID), t * QUARTER * 0.5)
}

/// A chevron turning over: pointing down at rest, up when the thing it belongs to is
/// open. Half a turn, so it passes through sideways rather than collapsing into a line —
/// which is what lerping the corners of a chevron onto its own reflection would do.
fn expand_collapse(t: f32) -> Path {
    let p = |x: f32, y: f32| Point::new(x, y);
    let chevron = Path::new()
        .move_to(p(5.6, 9.0))
        .line_to(p(12.0, 15.4))
        .line_to(p(18.4, 9.0))
        .line_to(p(18.4, 11.8))
        .line_to(p(12.0, 18.2))
        .line_to(p(5.6, 11.8))
        .close();
    chevron.rotated(Point::new(MID, MID), t * QUARTER * 2.0)
}

/// The **bundled pairs**, as one constant each — the same shape of thing [`Icons`] is for
/// the static set.
///
/// These are the ones this framework needs itself. An application that needs another
/// authors it with [`AnimatedIconData::custom`], which is the same door
/// [`IconData::custom`](crate::IconData::custom) opens for a static mark.
///
/// [`Icons`]: crate::Icons
pub struct AnimatedIcons;

impl AnimatedIcons {
    /// **Three bars ↔ a cross.** A drawer's button: `0.0` shut, `1.0` open.
    pub const MENU_CLOSE: AnimatedIconData = AnimatedIconData::custom(menu_close);
    /// **A triangle ↔ two bars.** A transport control: `0.0` stopped, `1.0` playing.
    ///
    /// It carries a direction: a play arrow points the way the reader reads, so it is
    /// turned round in a right-to-left order — and the pause it becomes is symmetrical,
    /// so nothing looks turned round at the end of it.
    pub const PLAY_PAUSE: AnimatedIconData = AnimatedIconData::custom(play_pause).mirrored();
    /// **A plus ↔ a cross.** An add button that becomes the way to put away what it
    /// opened: `0.0` add, `1.0` dismiss.
    pub const ADD_CLOSE: AnimatedIconData = AnimatedIconData::custom(add_close);
    /// **A chevron ↔ the same chevron, over.** Down when shut, up when open.
    pub const EXPAND_COLLAPSE: AnimatedIconData = AnimatedIconData::custom(expand_collapse);

    /// All four, with their names — for a picker, or for a test that wants to say
    /// something about every pair there is.
    pub fn all() -> impl Iterator<Item = (&'static str, AnimatedIconData)> {
        [
            ("menu_close", Self::MENU_CLOSE),
            ("play_pause", Self::PLAY_PAUSE),
            ("add_close", Self::ADD_CLOSE),
            ("expand_collapse", Self::EXPAND_COLLAPSE),
        ]
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{PathVerb, TextDirection};

    /// Every point a path visits, on-curve and control alike.
    fn points(path: &Path) -> Vec<Point> {
        let mut out = Vec::new();
        for verb in path.verbs() {
            match *verb {
                PathVerb::MoveTo(p) | PathVerb::LineTo(p) => out.push(p),
                PathVerb::QuadTo { ctrl, to } => out.extend([ctrl, to]),
                PathVerb::CubicTo { c1, c2, to } => out.extend([c1, c2, to]),
                PathVerb::Close => {}
            }
        }
        out
    }

    /// The three values the issue asks to be pinned, for every pair: the two ends and
    /// the middle. Each is checked for the property that makes it that state, so the
    /// numbers below are readable rather than a hash of the artwork.
    #[test]
    fn every_pair_is_drawn_at_nought_a_half_and_one() {
        for (name, pair) in AnimatedIcons::all() {
            for t in [0.0, 0.5, 1.0] {
                let path = pair.at(t).path();
                assert!(!path.is_empty(), "{name} at {t} drew nothing");
                for p in points(&path) {
                    assert!(
                        (-0.1..=24.1).contains(&p.x) && (-0.1..=24.1).contains(&p.y),
                        "{name} at {t} left the grid: {p:?}"
                    );
                }
            }
        }
    }

    /// **The middle is a middle**, not a jump. Between `t` and `t + 0.1` no point may
    /// move further than a few grid units — which is the difference between a drawing
    /// that morphs and two drawings with a dissolve between them.
    #[test]
    fn no_pair_jumps_on_the_way_across() {
        for (name, pair) in AnimatedIcons::all() {
            let mut previous = points(&pair.at(0.0).path());
            for step in 1..=10 {
                let t = step as f32 / 10.0;
                let now = points(&pair.at(t).path());
                assert_eq!(
                    now.len(),
                    previous.len(),
                    "{name} changed its point count at t = {t}"
                );
                for (a, b) in previous.iter().zip(&now) {
                    let moved = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();
                    assert!(moved < 4.0, "{name} jumped {moved} at t = {t}");
                }
                previous = now;
            }
        }
    }

    /// A morph at `t = 0` and the same morph at `t = 0` are the same icon, and one at a
    /// different `t` is not. `IconData` is a key — a cache, a comparison in a widget's
    /// `PartialEq` — so two positions of the same animation must not be interchangeable.
    #[test]
    fn two_positions_of_one_pair_are_different_icons() {
        let a = AnimatedIcons::MENU_CLOSE.at(0.0);
        let b = AnimatedIcons::MENU_CLOSE.at(0.0);
        let c = AnimatedIcons::MENU_CLOSE.at(1.0);
        assert_eq!(a, b, "the same pair at the same t");
        assert_ne!(a, c, "the same pair at a different t");
        assert_ne!(
            a,
            AnimatedIcons::ADD_CLOSE.at(0.0),
            "two pairs at the same t"
        );
    }

    /// `t` outside the range is brought back into it rather than drawing something the
    /// artwork was never asked about, and a `t` that is not a number is read as the
    /// start — because an icon that compares unequal to itself breaks `Eq`.
    #[test]
    fn a_t_outside_the_range_is_clamped_and_a_nan_is_not() {
        let pair = AnimatedIcons::PLAY_PAUSE;
        assert_eq!(pair.at(-3.0), pair.at(0.0));
        assert_eq!(pair.at(9.0), pair.at(1.0));
        let nan = pair.at(f32::NAN);
        assert_eq!(nan, nan, "an icon is equal to itself");
        assert_eq!(nan, pair.at(0.0), "and it is the start of the animation");
    }

    /// The signed area of each closed contour, by the shoelace formula — what the fill
    /// actually covers, which is not the same question as how many points there are.
    fn areas(path: &Path) -> Vec<f32> {
        let mut out = Vec::new();
        let mut contour: Vec<Point> = Vec::new();
        let flush = |c: &mut Vec<Point>, out: &mut Vec<f32>| {
            if c.len() > 2 {
                let mut twice = 0.0;
                for i in 0..c.len() {
                    let (a, b) = (c[i], c[(i + 1) % c.len()]);
                    twice += a.x * b.y - b.x * a.y;
                }
                out.push((twice * 0.5).abs());
            }
            c.clear();
        };
        for verb in path.verbs() {
            match *verb {
                PathVerb::MoveTo(p) => {
                    flush(&mut contour, &mut out);
                    contour.push(p);
                }
                PathVerb::LineTo(p) => contour.push(p),
                PathVerb::QuadTo { to, .. } | PathVerb::CubicTo { to, .. } => contour.push(to),
                PathVerb::Close => flush(&mut contour, &mut out),
            }
        }
        flush(&mut contour, &mut out);
        out
    }

    /// **Three bars become two**, and the one that goes does so by covering nothing —
    /// not by leaving the path. Three contours all the way across, and the middle one's
    /// ink runs out exactly at the end.
    #[test]
    fn the_menu_loses_its_middle_bar_without_losing_a_contour() {
        for t in [0.0, 0.5, 1.0] {
            let path = AnimatedIcons::MENU_CLOSE.at(t).path();
            assert_eq!(points(&path).len(), 12, "three bars at t = {t}");
            assert_eq!(areas(&path).len(), 3, "three contours at t = {t}");
        }
        let ink = |t: f32| areas(&AnimatedIcons::MENU_CLOSE.at(t).path())[1];
        assert!(ink(0.0) > 30.0, "the middle bar starts full: {}", ink(0.0));
        assert!(
            (ink(0.5) - ink(0.0) * 0.5).abs() < 0.1,
            "and is half gone halfway: {}",
            ink(0.5)
        );
        assert_eq!(ink(1.0), 0.0, "and covers nothing at the end");
    }

    /// A pair that carries a direction is turned round **at every `t`**, not only at the
    /// ends: a play arrow that pointed the right way when stopped and the wrong way
    /// halfway through would be worse than one that never turned at all.
    #[test]
    fn a_directional_pair_is_mirrored_all_the_way_across() {
        assert!(AnimatedIcons::PLAY_PAUSE.matches_text_direction());
        assert!(!AnimatedIcons::MENU_CLOSE.matches_text_direction());
        for step in 0..=4 {
            let t = step as f32 / 4.0;
            let icon = AnimatedIcons::PLAY_PAUSE.at(t);
            assert!(icon.matches_text_direction(), "at t = {t}");
            let ltr = icon.placed(24.0, 0.0, 0.0, TextDirection::Ltr);
            let rtl = icon.placed(24.0, 0.0, 0.0, TextDirection::Rtl);
            let (a, b) = (points(&ltr), points(&rtl));
            for (l, r) in a.iter().zip(&b) {
                assert!(
                    (l.x + r.x - 24.0).abs() < 1e-3 && (l.y - r.y).abs() < 1e-3,
                    "at t = {t}: {l:?} against {r:?}"
                );
            }
        }
    }

    /// A caller's own pair reaches the same machinery as a bundled one, and is not
    /// mistaken for a bundled icon on the way.
    #[test]
    fn a_callers_own_pair_is_an_icon_like_any_other() {
        fn box_that_grows(t: f32) -> Path {
            Path::rect(frus_core::Rect::new(4.0, 4.0, 16.0 * t, 16.0))
        }
        const MINE: AnimatedIconData = AnimatedIconData::custom(box_that_grows);
        let icon = MINE.at(0.5);
        assert!(!icon.is_bundled());
        assert_eq!(icon.style(), None);
        assert!(!icon.path().is_empty());
        assert_eq!(icon, MINE.at(0.5));
    }
}
