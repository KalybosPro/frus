//! **Gradients**, as the reference has them (`painting/gradient.dart`, milestone 644): a
//! linear one between two points, a radial one round a centre (with an optional focal
//! point), and a sweep round a centre. Each runs through a list of colours at optional
//! stops, and says what happens past its ends ([`TileMode`]).
//!
//! The geometry is the reference's, **relative to the box** it fills: points are
//! alignments (`-1` to `1` across the box), a radius is a fraction of the box's shorter side,
//! and angles are radians clockwise from the right. The renderer turns them into pixels
//! for the box it paints, so a gradient described once fills any box the same way.

use crate::{Alignment, AlignmentGeometry, Color, TextDirection};

/// What a gradient does past its ends (`dart:ui`'s `TileMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TileMode {
    /// The end colours carry on. The default.
    #[default]
    Clamp,
    /// The gradient starts again.
    Repeated,
    /// The gradient comes back the other way, and so on.
    Mirror,
    /// Nothing past the ends.
    Decal,
}

/// **A gradient along a line**, from `begin` to `end` (`gradient.dart`'s
/// `LinearGradient`). Unset, from the centre of the left side to the centre of the right.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    /// Where the first colour is.
    pub begin: AlignmentGeometry,
    /// Where the last colour is.
    pub end: AlignmentGeometry,
    /// The colours, first to last.
    pub colors: Vec<Color>,
    /// Where each colour is, from `0` to `1`, as many as there are colours; `None` spreads
    /// them evenly.
    pub stops: Option<Vec<f32>>,
    /// Past the ends.
    pub tile_mode: TileMode,
    /// A turn of the whole gradient about the box's centre, in radians clockwise — the
    /// reference's `GradientRotation`.
    pub rotation: f32,
}

/// **A gradient round a centre** (`gradient.dart`'s `RadialGradient`): the first colour at
/// `center`, the last on the circle `radius` round it. With a `focal` point, the first colour
/// is on the circle `focal_radius` round that point instead, and the gradient runs between
/// the two circles.
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    /// The centre. Unset, the box's.
    pub center: AlignmentGeometry,
    /// The radius, as a fraction of the box's shorter side. Unset, a half.
    pub radius: f32,
    /// The colours, first to last.
    pub colors: Vec<Color>,
    /// Where each colour is; `None` spreads them evenly.
    pub stops: Option<Vec<f32>>,
    /// Past the ends.
    pub tile_mode: TileMode,
    /// Where the first colour's circle is centred, when not at `center`.
    pub focal: Option<AlignmentGeometry>,
    /// The first colour's circle's radius, as a fraction of the shorter side.
    pub focal_radius: f32,
    /// A turn of the whole gradient about the box's centre, in radians clockwise.
    pub rotation: f32,
}

/// **A gradient turning round a centre** (`gradient.dart`'s `SweepGradient`): from
/// `start_angle` to `end_angle`, in radians clockwise from the right.
#[derive(Clone, Debug, PartialEq)]
pub struct SweepGradient {
    /// The centre. Unset, the box's.
    pub center: AlignmentGeometry,
    /// Where the first colour is. Unset, `0`, the right.
    pub start_angle: f32,
    /// Where the last colour is. Unset, a whole turn.
    pub end_angle: f32,
    /// The colours, first to last.
    pub colors: Vec<Color>,
    /// Where each colour is; `None` spreads them evenly.
    pub stops: Option<Vec<f32>>,
    /// Past the ends.
    pub tile_mode: TileMode,
    /// A turn of the whole gradient about the box's centre, in radians clockwise.
    pub rotation: f32,
}

/// Any of the three.
#[derive(Clone, Debug, PartialEq)]
pub enum Gradient {
    /// Along a line.
    Linear(LinearGradient),
    /// Round a centre.
    Radial(RadialGradient),
    /// Turning round a centre.
    Sweep(SweepGradient),
}

fn physical(a: Alignment) -> AlignmentGeometry {
    AlignmentGeometry::Physical(a)
}

impl LinearGradient {
    /// From the centre of the left side to the centre of the right, through `colors`.
    pub fn new(colors: impl Into<Vec<Color>>) -> Self {
        Self {
            begin: physical(Alignment::CENTER_LEFT),
            end: physical(Alignment::CENTER_RIGHT),
            colors: colors.into(),
            stops: None,
            tile_mode: TileMode::Clamp,
            rotation: 0.0,
        }
    }

    /// Where the first and the last colours are.
    #[must_use]
    pub fn between(
        mut self,
        begin: impl Into<AlignmentGeometry>,
        end: impl Into<AlignmentGeometry>,
    ) -> Self {
        self.begin = begin.into();
        self.end = end.into();
        self
    }
}

impl RadialGradient {
    /// Centred, half the shorter side round, through `colors`.
    pub fn new(colors: impl Into<Vec<Color>>) -> Self {
        Self {
            center: physical(Alignment::CENTER),
            radius: 0.5,
            colors: colors.into(),
            stops: None,
            tile_mode: TileMode::Clamp,
            focal: None,
            focal_radius: 0.0,
            rotation: 0.0,
        }
    }
}

impl SweepGradient {
    /// Centred, a whole turn from the right, through `colors`.
    pub fn new(colors: impl Into<Vec<Color>>) -> Self {
        Self {
            center: physical(Alignment::CENTER),
            start_angle: 0.0,
            end_angle: std::f32::consts::TAU,
            colors: colors.into(),
            stops: None,
            tile_mode: TileMode::Clamp,
            rotation: 0.0,
        }
    }
}

impl From<LinearGradient> for Gradient {
    fn from(g: LinearGradient) -> Self {
        Gradient::Linear(g)
    }
}

impl From<RadialGradient> for Gradient {
    fn from(g: RadialGradient) -> Self {
        Gradient::Radial(g)
    }
}

impl From<SweepGradient> for Gradient {
    fn from(g: SweepGradient) -> Self {
        Gradient::Sweep(g)
    }
}

impl Gradient {
    /// The colours.
    pub fn colors(&self) -> &[Color] {
        match self {
            Gradient::Linear(g) => &g.colors,
            Gradient::Radial(g) => &g.colors,
            Gradient::Sweep(g) => &g.colors,
        }
    }

    fn colors_mut(&mut self) -> &mut Vec<Color> {
        match self {
            Gradient::Linear(g) => &mut g.colors,
            Gradient::Radial(g) => &mut g.colors,
            Gradient::Sweep(g) => &mut g.colors,
        }
    }

    /// The stops as given.
    pub fn stops(&self) -> Option<&[f32]> {
        match self {
            Gradient::Linear(g) => g.stops.as_deref(),
            Gradient::Radial(g) => g.stops.as_deref(),
            Gradient::Sweep(g) => g.stops.as_deref(),
        }
    }

    /// What happens past the ends.
    pub fn tile_mode(&self) -> TileMode {
        match self {
            Gradient::Linear(g) => g.tile_mode,
            Gradient::Radial(g) => g.tile_mode,
            Gradient::Sweep(g) => g.tile_mode,
        }
    }

    /// The turn about the box's centre, in radians clockwise.
    pub fn rotation(&self) -> f32 {
        match self {
            Gradient::Linear(g) => g.rotation,
            Gradient::Radial(g) => g.rotation,
            Gradient::Sweep(g) => g.rotation,
        }
    }

    /// The stops in use: the ones given when there is one per colour, otherwise the
    /// colours spread evenly from `0` to `1` (`gradient.dart`'s `_impliedStops`).
    pub fn resolved_stops(&self) -> Vec<f32> {
        let n = self.colors().len();
        match self.stops() {
            Some(stops) if stops.len() == n => stops.to_vec(),
            _ if n <= 1 => vec![0.0; n],
            _ => (0..n).map(|i| i as f32 / (n - 1) as f32).collect(),
        }
    }

    /// **The colour at `t`**, between the colours either side of it, the end colours held
    /// past the stops. What the renderer bakes into its ramp; `t` is already tiled.
    pub fn sample(&self, t: f32) -> Color {
        let colors = self.colors();
        match colors {
            [] => Color::TRANSPARENT,
            [only] => *only,
            _ => {
                let stops = self.resolved_stops();
                if t <= stops[0] {
                    return colors[0];
                }
                for i in 1..colors.len() {
                    if t <= stops[i] {
                        let span = stops[i] - stops[i - 1];
                        let k = if span > 0.0 {
                            (t - stops[i - 1]) / span
                        } else {
                            1.0
                        };
                        return colors[i - 1].lerp(colors[i], k);
                    }
                }
                colors[colors.len() - 1]
            }
        }
    }

    /// The same gradient with its start and end made physical for `direction`.
    #[must_use]
    pub fn resolve(&self, direction: TextDirection) -> Gradient {
        let fix = |a: AlignmentGeometry| physical(a.resolve(direction));
        match self {
            Gradient::Linear(g) => Gradient::Linear(LinearGradient {
                begin: fix(g.begin),
                end: fix(g.end),
                ..g.clone()
            }),
            Gradient::Radial(g) => Gradient::Radial(RadialGradient {
                center: fix(g.center),
                focal: g.focal.map(fix),
                ..g.clone()
            }),
            Gradient::Sweep(g) => Gradient::Sweep(SweepGradient {
                center: fix(g.center),
                ..g.clone()
            }),
        }
    }

    /// Every colour's opacity times `factor` — the reference's `Gradient.scale`, which is
    /// how a gradient on one side of a transition arrives or leaves.
    #[must_use]
    pub fn scale(&self, factor: f32) -> Gradient {
        let mut g = self.clone();
        for c in g.colors_mut() {
            *c = c.fade(factor);
        }
        g
    }

    /// The gradient `t` of the way to `other` (`gradient.dart`'s `Gradient.lerp`): two of
    /// the same kind with as many colours travel colour by colour, stop by stop and point
    /// by point; anything else fades one out and the other in.
    #[must_use]
    pub fn lerp(a: Option<&Gradient>, b: Option<&Gradient>, t: f32) -> Option<Gradient> {
        let mix = |x: f32, y: f32| x + (y - x) * t;
        let mix_a = |x: AlignmentGeometry, y: AlignmentGeometry| {
            let (x, y) = (x.resolve(TextDirection::Ltr), y.resolve(TextDirection::Ltr));
            physical(Alignment::new(mix(x.x, y.x), mix(x.y, y.y)))
        };
        let colors = |x: &[Color], y: &[Color]| -> Vec<Color> {
            x.iter().zip(y).map(|(p, q)| p.lerp(*q, t)).collect()
        };
        let stops = |x: &Gradient, y: &Gradient| -> Option<Vec<f32>> {
            if x.stops().is_none() && y.stops().is_none() {
                return None;
            }
            Some(
                x.resolved_stops()
                    .iter()
                    .zip(y.resolved_stops())
                    .map(|(p, q)| mix(*p, q))
                    .collect(),
            )
        };
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), Some(b)) if a.colors().len() == b.colors().len() => {
                let tile = if t < 0.5 {
                    a.tile_mode()
                } else {
                    b.tile_mode()
                };
                match (a, b) {
                    (Gradient::Linear(x), Gradient::Linear(y)) => {
                        Some(Gradient::Linear(LinearGradient {
                            begin: mix_a(x.begin, y.begin),
                            end: mix_a(x.end, y.end),
                            colors: colors(&x.colors, &y.colors),
                            stops: stops(a, b),
                            tile_mode: tile,
                            rotation: mix(x.rotation, y.rotation),
                        }))
                    }
                    (Gradient::Radial(x), Gradient::Radial(y)) => {
                        Some(Gradient::Radial(RadialGradient {
                            center: mix_a(x.center, y.center),
                            radius: mix(x.radius, y.radius),
                            colors: colors(&x.colors, &y.colors),
                            stops: stops(a, b),
                            tile_mode: tile,
                            focal: match (x.focal, y.focal) {
                                (None, None) => None,
                                (p, q) => Some(mix_a(p.unwrap_or(x.center), q.unwrap_or(y.center))),
                            },
                            focal_radius: mix(x.focal_radius, y.focal_radius),
                            rotation: mix(x.rotation, y.rotation),
                        }))
                    }
                    (Gradient::Sweep(x), Gradient::Sweep(y)) => {
                        Some(Gradient::Sweep(SweepGradient {
                            center: mix_a(x.center, y.center),
                            start_angle: mix(x.start_angle, y.start_angle),
                            end_angle: mix(x.end_angle, y.end_angle),
                            colors: colors(&x.colors, &y.colors),
                            stops: stops(a, b),
                            tile_mode: tile,
                            rotation: mix(x.rotation, y.rotation),
                        }))
                    }
                    _ => Some(if t < 0.5 {
                        a.scale(1.0 - 2.0 * t)
                    } else {
                        b.scale(2.0 * t - 1.0)
                    }),
                }
            }
            (Some(a), Some(b)) => Some(if t < 0.5 {
                a.scale(1.0 - 2.0 * t)
            } else {
                b.scale(2.0 * t - 1.0)
            }),
        }
    }
}

/// Tiles `t` past the gradient's ends (`dart:ui`'s `TileMode`): `None` where nothing is
/// drawn.
pub fn tile(t: f32, mode: TileMode) -> Option<f32> {
    match mode {
        TileMode::Clamp => Some(t.clamp(0.0, 1.0)),
        TileMode::Repeated => Some(t - t.floor()),
        TileMode::Mirror => {
            let m = t.rem_euclid(2.0);
            Some(if m <= 1.0 { m } else { 2.0 - m })
        }
        TileMode::Decal => (0.0..=1.0).contains(&t).then_some(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);

    /// **Colours spread evenly unless told**, and between two stops a colour is the mix of
    /// its neighbours; past the stops, the end colours.
    #[test]
    fn colours_are_sampled_between_their_stops() {
        let even = Gradient::from(LinearGradient::new(vec![RED, GREEN, BLUE]));
        assert_eq!(even.resolved_stops(), vec![0.0, 0.5, 1.0]);
        assert_eq!(even.sample(0.0), RED);
        assert_eq!(even.sample(0.5), GREEN);
        assert_eq!(even.sample(0.25), RED.lerp(GREEN, 0.5));
        assert_eq!(even.sample(1.0), BLUE);

        let mut stopped = LinearGradient::new(vec![RED, BLUE]);
        stopped.stops = Some(vec![0.2, 0.6]);
        let stopped = Gradient::from(stopped);
        assert_eq!(stopped.sample(0.1), RED, "held before the first stop");
        let mid = stopped.sample(0.4);
        assert!(
            (mid.r - 0.5).abs() < 1e-6 && (mid.b - 0.5).abs() < 1e-6,
            "half way between the stops: {mid:?}"
        );
        assert_eq!(stopped.sample(0.9), BLUE, "and after the last");

        // Stops that do not match the colours are not used.
        let mut wrong = LinearGradient::new(vec![RED, GREEN, BLUE]);
        wrong.stops = Some(vec![0.0, 1.0]);
        assert_eq!(Gradient::from(wrong).resolved_stops(), vec![0.0, 0.5, 1.0]);
    }

    /// **Past the ends**: clamped, repeated, mirrored, or nothing.
    #[test]
    fn the_tile_modes() {
        assert_eq!(tile(1.4, TileMode::Clamp), Some(1.0));
        assert_eq!(tile(-0.3, TileMode::Clamp), Some(0.0));
        assert!((tile(1.25, TileMode::Repeated).unwrap() - 0.25).abs() < 1e-6);
        assert!((tile(1.25, TileMode::Mirror).unwrap() - 0.75).abs() < 1e-6);
        assert!((tile(-0.25, TileMode::Mirror).unwrap() - 0.25).abs() < 1e-6);
        assert_eq!(tile(1.25, TileMode::Decal), None);
        assert_eq!(tile(0.5, TileMode::Decal), Some(0.5));
    }

    /// **The reference's defaults.**
    #[test]
    fn the_defaults_are_the_reference_s() {
        let l = LinearGradient::new(vec![RED, BLUE]);
        assert_eq!(l.begin, physical(Alignment::CENTER_LEFT));
        assert_eq!(l.end, physical(Alignment::CENTER_RIGHT));
        let r = RadialGradient::new(vec![RED, BLUE]);
        assert_eq!((r.radius, r.focal, r.focal_radius), (0.5, None, 0.0));
        let s = SweepGradient::new(vec![RED, BLUE]);
        assert_eq!((s.start_angle, s.end_angle), (0.0, std::f32::consts::TAU));
        assert_eq!(l.tile_mode, TileMode::Clamp);
    }

    /// **Two of a kind travel point by point and colour by colour; a gradient on one side
    /// only fades.**
    #[test]
    fn gradients_lerp_as_the_reference_s() {
        let a = Gradient::from(LinearGradient::new(vec![RED, RED]));
        let b = Gradient::from(
            LinearGradient::new(vec![BLUE, BLUE])
                .between(Alignment::TOP_CENTER, Alignment::BOTTOM_CENTER),
        );
        let mid = Gradient::lerp(Some(&a), Some(&b), 0.5).unwrap();
        match &mid {
            Gradient::Linear(g) => {
                assert_eq!(g.colors[0], RED.lerp(BLUE, 0.5));
                assert_eq!(g.begin, physical(Alignment::new(-0.5, -0.5)));
            }
            other => panic!("{other:?}"),
        }
        let fading = Gradient::lerp(Some(&a), None, 0.25).unwrap();
        assert!((fading.colors()[0].a - 0.75).abs() < 1e-6);
        // Of different kinds: one out, then the other in.
        let sweep = Gradient::from(SweepGradient::new(vec![GREEN, GREEN]));
        assert!(matches!(
            Gradient::lerp(Some(&a), Some(&sweep), 0.25),
            Some(Gradient::Linear(_))
        ));
        assert!(matches!(
            Gradient::lerp(Some(&a), Some(&sweep), 0.75),
            Some(Gradient::Sweep(_))
        ));
    }

    /// **A gradient painted at an opacity carries it in its colours**, as a fill does: a
    /// decoration fading in or out fades its gradient too.
    #[test]
    fn a_faded_gradient_rectangle_carries_the_fade() {
        let mut scene = crate::Scene::new();
        scene.shaded_rect(
            crate::Rect::new(0.0, 0.0, 10.0, 10.0),
            Gradient::from(LinearGradient::new(vec![RED, BLUE])),
            0.5,
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
        match &scene.primitives()[0] {
            crate::Primitive::Rect {
                shader: Some(g),
                color,
                ..
            } => {
                assert_eq!(g.colors()[1].a, 0.5);
                assert_eq!(
                    *color,
                    RED.fade(0.5),
                    "the first colour, for what reads a colour"
                );
            }
            other => panic!("{other:?}"),
        }
    }
}
