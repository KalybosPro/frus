//! A small **set of vector icons**, each defined as a filled [`Path`] (a
//! silhouette, Material Icons style) on a **24×24** grid. The [`crate::Icon`]
//! widget scales the path to its real size and colors it from the theme. Every
//! icon is meant to be *filled* (the non-zero rule).
//!
//! Adding an icon = adding a variant to [`IconName`] and its arm in
//! [`IconName::path`]. The coordinates are in units of the 24×24 grid.

use std::f32::consts::{FRAC_PI_2, PI};

use frus_core::{Path, Point, Rect};

/// One icon from the bundled set. Each variant returns a `24×24` normalised [`Path`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
    /// Coche de validation.
    Check,
    /// Croix de fermeture.
    Close,
    /// Signe plus (ajouter).
    Add,
    /// Three bars (a "hamburger" menu).
    Menu,
    /// A solid five-pointed star.
    Star,
    /// A heart.
    Heart,
    /// Disque plein.
    Circle,
    /// A solid square.
    Square,
    /// A "play" triangle pointing right.
    Play,
    /// A chevron pointing left.
    ChevronLeft,
    /// A chevron pointing right.
    ChevronRight,
    /// An eye (visible): a ring-shaped eye outline + a pupil — reveal a password.
    Eye,
    /// A crossed-out eye (hidden): the eye with a diagonal through it — hide.
    EyeOff,
}

impl IconName {
    /// The icon's vector path, on the `24×24` grid, ready to be scaled.
    pub fn path(self) -> Path {
        match self {
            IconName::Check => polygon(&[
                (9.55, 17.05),
                (4.0, 11.5),
                (5.8, 9.7),
                (9.55, 13.45),
                (18.2, 4.8),
                (20.0, 6.6),
            ]),
            IconName::Close => cross_x(),
            IconName::Add => plus(),
            IconName::Menu => bars(),
            IconName::Star => star(Point::new(12.0, 12.5), 10.0, 4.2, 5),
            IconName::Heart => heart(),
            IconName::Circle => Path::circle(Point::new(12.0, 12.0), 9.0),
            IconName::Square => Path::rect(Rect::new(3.0, 3.0, 18.0, 18.0)),
            IconName::Play => polygon(&[(7.0, 4.0), (20.0, 12.0), (7.0, 20.0)]),
            IconName::ChevronLeft => polygon(&[
                (16.0, 5.8),
                (14.2, 4.0),
                (6.2, 12.0),
                (14.2, 20.0),
                (16.0, 18.2),
                (9.8, 12.0),
            ]),
            IconName::ChevronRight => polygon(&[
                (8.0, 4.0),
                (16.0, 12.0),
                (8.0, 20.0),
                (6.2, 18.2),
                (12.4, 12.0),
                (6.2, 5.8),
            ]),
            IconName::Eye => eye(false),
            IconName::EyeOff => eye(true),
        }
    }
}

/// The "eye" icon: an almond-shaped **ring** (an outer contour plus an inner contour of the
/// **opposite** direction, which hollows out the opening by the non-zero rule) and a solid
/// **pupil** at the centre. If `off`, a diagonal crosses the eye out (hidden).
///
/// The opening is guaranteed *whatever* the absolute drawing direction is: the inner contour
/// is the outer almond **walked backwards**, so of opposite winding — their contributions
/// cancel out (0 = transparent) inside the opening, while the pupil adds a non-zero winding
/// there (solid).
fn eye(off: bool) -> Path {
    // An almond = two quadratic curves (bulging up, then down) between two endpoints.
    // `rev` flips the walking direction (down then up) to get the opposite winding.
    let almond = |hw: f32, top_ctrl: f32, bot_ctrl: f32, rev: bool| {
        let (l, r) = (Point::new(12.0 - hw, 12.0), Point::new(12.0 + hw, 12.0));
        let (top, bot) = (Point::new(12.0, top_ctrl), Point::new(12.0, bot_ctrl));
        if rev {
            (l, bot, r, top)
        } else {
            (l, top, r, bot)
        }
    };
    // Contour externe (amande large) : gauche → (haut) → droite → (bas) → gauche.
    let (l1, c1a, r1, c1b) = almond(10.0, -2.0, 26.0, false);
    let mut path = Path::new()
        .move_to(l1)
        .quad_to(c1a, r1)
        .quad_to(c1b, l1)
        .close();
    // The inner contour (a narrow almond), walked backwards → hollows out the opening.
    let (l2, c2a, r2, c2b) = almond(7.5, 2.0, 22.0, true);
    path = path.move_to(l2).quad_to(c2a, r2).quad_to(c2b, l2).close();
    // Pupille pleine.
    let pupil = Path::circle(Point::new(12.0, 12.0), 3.0);
    for v in pupil.verbs() {
        path = push_verb(path, *v);
    }
    if off {
        // A diagonal bar (a thin rectangle) from bottom-left to top-right.
        path = path
            .move_to(Point::new(5.2, 19.3))
            .line_to(Point::new(20.2, 6.3))
            .line_to(Point::new(18.8, 4.7))
            .line_to(Point::new(3.8, 17.7))
            .close();
    }
    path
}

/// Re-emits a [`PathVerb`] into the [`Path`] *builder*, to copy an existing subpath.
fn push_verb(path: Path, verb: frus_core::PathVerb) -> Path {
    use frus_core::PathVerb::*;
    match verb {
        MoveTo(p) => path.move_to(p),
        LineTo(p) => path.line_to(p),
        QuadTo { ctrl, to } => path.quad_to(ctrl, to),
        CubicTo { c1, c2, to } => path.cubic_to(c1, c2, to),
        Close => path.close(),
    }
}

/// Builds a closed polygon from a list of `(x, y)` vertices.
fn polygon(points: &[(f32, f32)]) -> Path {
    let mut path = Path::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        let p = Point::new(x, y);
        path = if i == 0 {
            path.move_to(p)
        } else {
            path.line_to(p)
        };
    }
    path.close()
}

/// A plus sign: a cross-shaped polygon (arms 4 thick, centred on 12).
fn plus() -> Path {
    polygon(&[
        (10.0, 4.0),
        (14.0, 4.0),
        (14.0, 10.0),
        (20.0, 10.0),
        (20.0, 14.0),
        (14.0, 14.0),
        (14.0, 20.0),
        (10.0, 20.0),
        (10.0, 14.0),
        (4.0, 14.0),
        (4.0, 10.0),
        (10.0, 10.0),
    ])
}

/// A close cross: two diagonal bars, as two filled subpaths.
fn cross_x() -> Path {
    Path::new()
        // Diagonale ↘
        .move_to(Point::new(5.0, 6.4))
        .line_to(Point::new(6.4, 5.0))
        .line_to(Point::new(19.0, 17.6))
        .line_to(Point::new(17.6, 19.0))
        .close()
        // Diagonale ↙
        .move_to(Point::new(17.6, 5.0))
        .line_to(Point::new(19.0, 6.4))
        .line_to(Point::new(6.4, 19.0))
        .line_to(Point::new(5.0, 17.6))
        .close()
}

/// A "hamburger" menu: three horizontal bars, as three subpaths.
fn bars() -> Path {
    let mut path = Path::new();
    for y in [5.0_f32, 11.0, 17.0] {
        path = path
            .move_to(Point::new(3.0, y))
            .line_to(Point::new(21.0, y))
            .line_to(Point::new(21.0, y + 2.0))
            .line_to(Point::new(3.0, y + 2.0))
            .close();
    }
    path
}

/// A star with `points` points (outer and inner radii), apex upwards.
fn star(center: Point, outer: f32, inner: f32, points: usize) -> Path {
    let mut path = Path::new();
    let step = PI / points as f32;
    for i in 0..(points * 2) {
        let r = if i % 2 == 0 { outer } else { inner };
        let a = -FRAC_PI_2 + step * i as f32;
        let p = Point::new(center.x + r * a.cos(), center.y + r * a.sin());
        path = if i == 0 {
            path.move_to(p)
        } else {
            path.line_to(p)
        };
    }
    path.close()
}

/// A heart, drawn with cubic curves (two symmetric lobes).
fn heart() -> Path {
    Path::new()
        .move_to(Point::new(12.0, 20.0))
        .cubic_to(
            Point::new(7.0, 16.0),
            Point::new(3.0, 12.5),
            Point::new(3.0, 8.5),
        )
        .cubic_to(
            Point::new(3.0, 6.0),
            Point::new(5.0, 4.0),
            Point::new(7.5, 4.0),
        )
        .cubic_to(
            Point::new(9.5, 4.0),
            Point::new(11.2, 5.3),
            Point::new(12.0, 7.0),
        )
        .cubic_to(
            Point::new(12.8, 5.3),
            Point::new(14.5, 4.0),
            Point::new(16.5, 4.0),
        )
        .cubic_to(
            Point::new(19.0, 4.0),
            Point::new(21.0, 6.0),
            Point::new(21.0, 8.5),
        )
        .cubic_to(
            Point::new(21.0, 12.5),
            Point::new(17.0, 16.0),
            Point::new(12.0, 20.0),
        )
        .close()
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::PathVerb;

    #[test]
    fn every_icon_yields_a_non_empty_path() {
        for name in [
            IconName::Check,
            IconName::Close,
            IconName::Add,
            IconName::Menu,
            IconName::Star,
            IconName::Heart,
            IconName::Circle,
            IconName::Square,
            IconName::Play,
            IconName::ChevronLeft,
            IconName::ChevronRight,
            IconName::Eye,
            IconName::EyeOff,
        ] {
            let path = name.path();
            assert!(!path.is_empty(), "{name:?} devrait produire un chemin");
            assert!(
                matches!(path.verbs().first(), Some(PathVerb::MoveTo(_))),
                "{name:?} devrait commencer par un MoveTo"
            );
        }
    }

    #[test]
    fn eye_is_a_ring_with_a_pupil_and_off_adds_a_slash() {
        // Eye = outer contour + inner contour (opposite) + pupil = 3 closed subpaths.
        let subpaths = |name: IconName| {
            name.path()
                .verbs()
                .iter()
                .filter(|v| matches!(v, PathVerb::Close))
                .count()
        };
        assert_eq!(subpaths(IconName::Eye), 3, "ring (2 almonds) + pupil");
        // The crossed-out eye adds the diagonal.
        assert_eq!(subpaths(IconName::EyeOff), 4, "eye + diagonal bar");
    }

    #[test]
    fn star_has_ten_outline_points() {
        // 5 points → 10 vertices (outer/inner alternating) + move + close.
        let star = IconName::Star.path();
        let lines = star
            .verbs()
            .iter()
            .filter(|v| matches!(v, PathVerb::LineTo(_) | PathVerb::MoveTo(_)))
            .count();
        assert_eq!(lines, 10);
    }

    #[test]
    fn menu_has_three_subpaths() {
        let closes = IconName::Menu
            .path()
            .verbs()
            .iter()
            .filter(|v| matches!(v, PathVerb::Close))
            .count();
        assert_eq!(closes, 3, "three bars = three closed subpaths");
    }
}
