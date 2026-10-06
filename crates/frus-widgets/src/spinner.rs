//! [`CircularProgressIndicator`]: the reference's circular progress indicator — an arc,
//! determinate or spinning (milestone 630) — and the ring of dots the pull-to-refresh
//! indicator still draws.

use frus_core::{Color, Curve, Path, Point, Rect, Scene, Stroke, StrokeCap};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Number of dots in the ring.
const DOTS: usize = 8;

/// How the ring reads: **turning**, which means work is happening, or **filling**,
/// which means a gesture is part of the way to asking for some.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RingMode {
    /// A bright head advancing, the dots behind it fading out. `head` is in turns.
    Spinning { head: f32 },
    /// The first `progress` of the ring, clockwise from the top. `0..=1`.
    Filling { progress: f32 },
}

/// Draws the framework's activity ring: `DOTS` dots on a circle of `radius` about
/// `(cx, cy)`, each `dot` px in radius.
///
/// The pull-to-refresh indicator's; the circular progress indicator draws the reference's
/// arc since milestone 630.
pub(crate) fn paint_activity_ring(
    scene: &mut Scene,
    cx: f32,
    cy: f32,
    radius: f32,
    dot: f32,
    color: Color,
    mode: RingMode,
) {
    for i in 0..DOTS {
        let fraction = i as f32 / DOTS as f32;
        let angle = fraction * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let alpha = match mode {
            RingMode::Spinning { head } => {
                let head = head.fract() * DOTS as f32;
                // The angular distance behind the head: 0 is the head, about 1 the tail.
                let behind = (i as f32 - head).rem_euclid(DOTS as f32) / DOTS as f32;
                0.15 + 0.85 * (1.0 - behind)
            }
            // A partial dot at the leading edge, so the fill grows smoothly instead of
            // clicking round one eighth at a time.
            RingMode::Filling { progress } => ((progress - fraction) * DOTS as f32).clamp(0.0, 1.0),
        };
        if alpha <= 0.0 {
            continue;
        }
        let px = cx + radius * angle.cos();
        let py = cy + radius * angle.sin();
        scene.draw_rect(
            Rect::new(px - dot, py - dot, dot * 2.0, dot * 2.0),
            color.fade(alpha),
            dot,
            0.0,
            Color::TRANSPARENT,
        );
    }
}

/// One cycle of the indeterminate arc's ends, in seconds (`progress_indicator.dart:1057`:
/// the controller's 1333 × 2222 ms run is 2222 of these).
const PATH_PERIOD: f32 = 1.333;
/// One turn of the whole indeterminate arc, in seconds (1333 of these per run).
const ROTATION_PERIOD: f32 = 2.222;
/// The reference never sweeps a whole circle, which its canvas would not draw
/// (`progress_indicator.dart:705`).
const EPSILON: f32 = 0.001;
/// Straight up, where every arc starts (`progress_indicator.dart:708`).
const START_ANGLE: f32 = -std::f32::consts::FRAC_PI_2;

/// A circular progress indicator: an arc around a circle — **determinate**, sweeping
/// clockwise from the top to its value, or **indeterminate**, an arc that grows, shrinks
/// and turns while the length of the work is unknown (`progress_indicator.dart:863`).
///
/// The look is the reference's **default** one, its 2023 appearance: a 36 px circle, a
/// 4 px stroke centred on it, flat ends, no track. [`year2023(false)`](Self::year2023) is
/// its newer one: a 40 px circle inside 4 px of padding, the stroke inside it, round ends,
/// and for a determinate indicator a track with a gap at each end of the arc.
pub struct CircularProgressIndicator {
    value: Option<f32>,
    size: Option<f32>,
    color: Option<Color>,
    track_color: Option<Color>,
    stroke_width: Option<f32>,
    stroke_align: Option<f32>,
    stroke_cap: Option<StrokeCap>,
    track_gap: Option<f32>,
    padding: Option<f32>,
    year2023: Option<bool>,
    semantics_label: Option<String>,
    semantics_value: Option<String>,
}

impl CircularProgressIndicator {
    /// An **indeterminate** indicator: the work is under way and how long it will take is
    /// not known — the reference's default.
    pub fn new() -> Self {
        Self::with_value(None)
    }

    /// A **determinate** indicator, its arc swept to `value`, clamped to `0.0..=1.0`.
    pub fn determinate(value: f32) -> Self {
        Self::with_value(Some(value))
    }

    /// An indicator at `value`, or indeterminate for `None` — the reference's nullable
    /// `value`.
    pub fn with_value(value: Option<f32>) -> Self {
        Self {
            value: value.map(|v| v.clamp(0.0, 1.0)),
            size: None,
            color: None,
            track_color: None,
            stroke_width: None,
            stroke_align: None,
            stroke_cap: None,
            track_gap: None,
            padding: None,
            year2023: None,
            semantics_label: None,
            semantics_value: None,
        }
    }

    /// **The side of the circle**, in logical pixels — the reference's `constraints`, as a
    /// square. Unset, the theme's, then 36 (2023 look) or 40 (newer).
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// **The colour of the arc**, over the theme's and `primary`.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// **The circle behind the arc** — the reference's `backgroundColor`. Unset, the
    /// theme's, then none in the 2023 look, `secondary_container` behind a determinate arc
    /// in the newer one.
    #[must_use]
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// **How thick the arc is.** Unset, the theme's, then four.
    #[must_use]
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = Some(width);
        self
    }

    /// **Where the line sits on the circle**: `-1` inside, `0` centred, `1` outside — the
    /// reference's `strokeAlignInside`, `Center` and `Outside`.
    #[must_use]
    pub fn stroke_align(mut self, align: f32) -> Self {
        self.stroke_align = Some(align);
        self
    }

    /// **How the arc's ends are drawn.**
    #[must_use]
    pub fn stroke_cap(mut self, cap: StrokeCap) -> Self {
        self.stroke_cap = Some(cap);
        self
    }

    /// **The gap between the arc and the track**, in the newer look.
    #[must_use]
    pub fn track_gap(mut self, gap: f32) -> Self {
        self.track_gap = Some(gap);
        self
    }

    /// **The room around the circle**, on every side.
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    /// **Which look**: the 2023 one (`true`, the default, as the reference's) or the newer
    /// one (`false`).
    #[must_use]
    pub fn year2023(mut self, year2023: bool) -> Self {
        self.year2023 = Some(year2023);
        self
    }

    /// **What the indicator is the progress of**, for whoever cannot see it.
    #[must_use]
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    /// **The value read out**, in place of the percentage.
    #[must_use]
    pub fn semantics_value(mut self, value: impl Into<String>) -> Self {
        self.semantics_value = Some(value.into());
        self
    }

    fn year2023_in(&self, theme: &Theme) -> bool {
        self.year2023
            .or(theme.widgets.progress.year2023)
            .unwrap_or(true)
    }

    /// The circle's side and the room around it: the caller's, the theme's, the reference's.
    fn extent(&self, theme: &Theme) -> (f32, f32) {
        let t = &theme.widgets.progress;
        let y2023 = self.year2023_in(theme);
        let size = self
            .size
            .or(t.circular_size)
            .unwrap_or(if y2023 { 36.0 } else { 40.0 });
        let padding = self
            .padding
            .or(t.circular_track_padding)
            .unwrap_or(if y2023 { 0.0 } else { 4.0 });
        (size, padding)
    }
}

impl Default for CircularProgressIndicator {
    fn default() -> Self {
        Self::new()
    }
}

/// The indeterminate arc's ends, how far it has been carried round, and how far turned, at
/// `time` seconds (`progress_indicator.dart:1060`): the head runs the first half of each
/// cycle on `fastOutSlowIn`, the tail the second half.
fn indeterminate_phase(time: f32) -> (f32, f32, f32, f32) {
    let path = (time / PATH_PERIOD).rem_euclid(1.0);
    let rotation = (time / ROTATION_PERIOD).rem_euclid(1.0);
    let half = |begin: f32| Curve::Interval {
        begin,
        end: begin + 0.5,
        inner: Box::new(Curve::Cubic {
            x1: 0.4,
            y1: 0.0,
            x2: 0.2,
            y2: 1.0,
        }),
    };
    (
        half(0.0).transform(path),
        half(0.5).transform(path),
        path,
        rotation,
    )
}

/// Where the arc starts and how far it sweeps, in radians (`progress_indicator.dart:679`).
fn arc(value: Option<f32>, time: f32) -> (f32, f32) {
    use std::f32::consts::{PI, TAU};
    match value {
        Some(v) => (START_ANGLE, v.clamp(0.0, 1.0) * (TAU - EPSILON)),
        None => {
            let (head, tail, offset, rotation) = indeterminate_phase(time);
            (
                START_ANGLE + tail * 1.5 * PI + rotation * TAU + offset * 0.5 * PI,
                (head * 1.5 * PI - tail * 1.5 * PI).max(EPSILON),
            )
        }
    }
}

/// An arc of `sweep` radians from `start`, around `center`.
fn arc_path(center: Point, radius: f32, start: f32, sweep: f32) -> Path {
    Path::new()
        .move_to(Point::new(
            center.x + radius * start.cos(),
            center.y + radius * start.sin(),
        ))
        .arc_to(center, radius, start, start + sweep)
}

impl<Msg> Widget<Msg> for CircularProgressIndicator {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let (size, padding) = self.extent(theme);
        let side = size + 2.0 * padding;
        Style {
            width: Dimension::Length(side),
            height: Dimension::Length(side),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    /// An indeterminate indicator moves for as long as it is shown; a determinate one only
    /// when its value changes.
    fn continuous(&self) -> bool {
        self.value.is_none()
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        use std::f32::consts::TAU;
        let t = &theme.widgets.progress;
        let y2023 = self.year2023_in(theme);
        let (_, padding) = self.extent(theme);
        let o = status.opacity;
        let stroke = self.stroke_width.or(t.stroke_width).unwrap_or(4.0);
        let align = self
            .stroke_align
            .or(t.stroke_align)
            .unwrap_or(if y2023 { 0.0 } else { -1.0 });
        let value_color = self.color.or(t.color).unwrap_or(theme.scheme.primary);
        let track_color = self
            .track_color
            .or(t.circular_track_color)
            .or((!y2023 && self.value.is_some()).then_some(theme.scheme.secondary_container));
        let gap = if y2023 {
            None
        } else {
            Some(self.track_gap.or(t.track_gap).unwrap_or(4.0))
        };
        let cap = self.stroke_cap.or(t.stroke_cap);

        // The circle the line is drawn on: the box less the padding, moved in or out by
        // the alignment (`progress_indicator.dart:719`).
        let inset = padding + stroke * 0.5 * -align;
        let w = (bounds.width - 2.0 * inset).max(0.0);
        let h = (bounds.height - 2.0 * inset).max(0.0);
        let center = Point::new(
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
        );
        let radius = w.min(h) * 0.5;
        if radius <= 0.0 {
            return;
        }

        if let Some(track) = track_color {
            let track_stroke =
                Stroke::new(track.fade(o), stroke).with_cap(cap.unwrap_or(StrokeCap::Round));
            match (gap, self.value) {
                // The track stops short of both ends of the arc, by the stroke and the gap
                // (`progress_indicator.dart:731`).
                (Some(gap), Some(v)) if gap > 0.0 && v > EPSILON => {
                    let start_gap = (stroke + gap) / radius;
                    let sweep = (TAU - v * TAU - 2.0 * start_gap).max(0.0);
                    if sweep > 0.0 {
                        let start = START_ANGLE + v * TAU + start_gap;
                        scene.paint_path(
                            &arc_path(center, radius, start, sweep),
                            None,
                            Some(track_stroke),
                        );
                    }
                }
                _ => scene.paint_path(
                    &arc_path(center, radius, 0.0, TAU - EPSILON),
                    None,
                    Some(track_stroke),
                ),
            }
        }

        let cap = match (y2023, cap, self.value) {
            (_, Some(cap), _) => cap,
            (true, None, None) => StrokeCap::Square,
            (true, None, Some(_)) => StrokeCap::Butt,
            (false, None, _) => StrokeCap::Round,
        };
        let (start, sweep) = arc(self.value, status.time);
        if self.value == Some(0.0) {
            return;
        }
        scene.paint_path(
            &arc_path(center, radius, start, sweep),
            None,
            Some(Stroke::new(value_color.fade(o), stroke).with_cap(cap)),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// A determinate indicator says its value from 0 to 100; an indeterminate one says only
    /// that it is busy (`progress_indicator.dart:146`).
    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let mut s = frus_core::SemanticsProperties::new(frus_core::Role::ProgressBar);
        if let Some(label) = &self.semantics_label {
            s = s.label(label.clone());
        }
        if let Some(value) = self.value {
            let pct = (value * 100.0).round();
            s = s.range(0.0, pct, 100.0);
            s = s.value(self.semantics_value.clone().unwrap_or(format!("{pct}")));
        } else if let Some(said) = &self.semantics_value {
            s = s.value(said.clone());
        }
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{PathVerb, Primitive};
    use std::f32::consts::{PI, TAU};

    struct Arc {
        start: Point,
        end: Point,
        color: Color,
        width: f32,
        cap: StrokeCap,
    }

    fn arcs(spinner: &CircularProgressIndicator, theme: &Theme, time: f32) -> Vec<Arc> {
        let side = match Widget::<()>::style_themed(spinner, theme).width {
            Dimension::Length(s) => s,
            _ => unreachable!(),
        };
        let mut scene = Scene::new();
        let status = Status {
            time,
            ..Default::default()
        };
        Widget::<()>::paint(
            spinner,
            Rect::new(0.0, 0.0, side, side),
            status,
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Path {
                    path,
                    stroke: Some(s),
                    ..
                } => {
                    let start = match path.verbs()[0] {
                        PathVerb::MoveTo(p) => p,
                        _ => unreachable!(),
                    };
                    let end = match path.verbs().last() {
                        Some(PathVerb::CubicTo { to, .. }) => *to,
                        _ => start,
                    };
                    Some(Arc {
                        start,
                        end,
                        color: s.color,
                        width: s.width,
                        cap: s.cap,
                    })
                }
                _ => None,
            })
            .collect()
    }

    fn near(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 0.05 && (a.y - b.y).abs() < 0.05
    }

    fn on_circle(c: f32, r: f32, angle: f32) -> Point {
        Point::new(c + r * angle.cos(), c + r * angle.sin())
    }

    /// **The default look is the reference's default**: a 36 px circle, a 4 px stroke
    /// centred on it, flat ends, no track — an arc, not a ring of dots
    /// (`progress_indicator.dart:1535`, `:754`).
    #[test]
    fn the_default_look_is_the_reference_s_default() {
        let theme = Theme::default();
        let quarter = CircularProgressIndicator::determinate(0.25);
        assert_eq!(
            Widget::<()>::style_themed(&quarter, &theme).width,
            Dimension::Length(36.0)
        );
        let a = arcs(&quarter, &theme, 0.0);
        assert_eq!(a.len(), 1, "one arc and no track");
        let (c, r) = (18.0, 18.0);
        assert!(near(a[0].start, on_circle(c, r, -PI / 2.0)), "from the top");
        assert!(
            near(
                a[0].end,
                on_circle(c, r, -PI / 2.0 + 0.25 * (TAU - EPSILON))
            ),
            "a quarter clockwise"
        );
        assert_eq!((a[0].width, a[0].cap), (4.0, StrokeCap::Butt));
        assert_eq!(a[0].color, theme.scheme.primary);
        // Spinning, the ends are square.
        let spinning = arcs(&CircularProgressIndicator::new(), &theme, 0.3);
        assert_eq!(spinning[0].cap, StrokeCap::Square);
    }

    /// **The newer look**: a 40 px circle in 4 px of padding, the stroke inside it, round
    /// ends, and behind a determinate arc a track that stops a stroke and a gap short of
    /// each end (`progress_indicator.dart:731`, `:1579`).
    #[test]
    fn the_newer_look_is_the_reference_s() {
        let theme = Theme::default();
        let half = CircularProgressIndicator::determinate(0.5).year2023(false);
        assert_eq!(
            Widget::<()>::style_themed(&half, &theme).width,
            Dimension::Length(48.0)
        );
        let a = arcs(&half, &theme, 0.0);
        assert_eq!(a.len(), 2, "a track and the arc");
        let (track, value) = (&a[0], &a[1]);
        // Inside: the line's centre is half a stroke in from the padded box.
        let (c, r) = (24.0, 20.0 - 2.0);
        assert!(near(value.start, on_circle(c, r, -PI / 2.0)));
        assert_eq!(value.cap, StrokeCap::Round);
        assert_eq!(track.color, theme.scheme.secondary_container);
        let gap = (4.0 + 4.0) / r;
        assert!(near(track.start, on_circle(c, r, -PI / 2.0 + PI + gap)));
        assert!(near(track.end, on_circle(c, r, -PI / 2.0 + TAU - gap)));
        // Spinning, there is no track.
        let spinning = arcs(
            &CircularProgressIndicator::new().year2023(false),
            &theme,
            0.3,
        );
        assert_eq!(spinning.len(), 1);
    }

    /// **The indeterminate arc follows the reference's timing**: its ends on
    /// `fastOutSlowIn` over 1.333 s halves, the whole turning once in 2.222 s
    /// (`progress_indicator.dart:1060`, `:679`).
    #[test]
    fn an_indeterminate_arc_runs_the_reference_s_timing() {
        let spinner = CircularProgressIndicator::new();
        assert!(Widget::<()>::continuous(&spinner));
        assert!(!Widget::<()>::continuous(
            &CircularProgressIndicator::determinate(0.3)
        ));
        let fast_out_slow_in = Curve::Cubic {
            x1: 0.4,
            y1: 0.0,
            x2: 0.2,
            y2: 1.0,
        };
        // A quarter of the way through a cycle: the head half way along its curve, the tail
        // not yet moving.
        let time = 0.25 * PATH_PERIOD;
        let (head, tail, offset, rotation) = indeterminate_phase(time);
        assert!((head - fast_out_slow_in.transform(0.5)).abs() < 1e-4);
        assert_eq!(tail, 0.0);
        assert!((offset - 0.25).abs() < 1e-4);
        assert!((rotation - time / ROTATION_PERIOD).abs() < 1e-4);
        let (start, sweep) = arc(None, time);
        let expect_start = START_ANGLE + rotation * TAU + offset * 0.5 * PI;
        assert!((start - expect_start).abs() < 1e-4);
        assert!((sweep - head * 1.5 * PI).abs() < 1e-4);
        // At the start of a cycle the arc is the smallest the reference draws.
        assert_eq!(arc(None, 0.0).1, EPSILON);
        // And the painted arc starts where the formula says.
        let a = arcs(&spinner, &Theme::default(), time);
        assert!(near(a[0].start, on_circle(18.0, 18.0, start)));
    }

    /// **Everything answers to the theme and to the caller.**
    #[test]
    fn an_indicator_answers_to_its_theme_and_to_its_caller() {
        let mut theme = Theme::default();
        theme.widgets.progress.color = Some(Color::rgb(0.9, 0.1, 0.1));
        theme.widgets.progress.circular_track_color = Some(Color::rgb(0.2, 0.2, 0.2));
        theme.widgets.progress.stroke_width = Some(6.0);
        theme.widgets.progress.stroke_cap = Some(StrokeCap::Round);
        theme.widgets.progress.circular_size = Some(50.0);
        let plain = CircularProgressIndicator::determinate(0.5);
        assert_eq!(
            Widget::<()>::style_themed(&plain, &theme).width,
            Dimension::Length(50.0)
        );
        let a = arcs(&plain, &theme, 0.0);
        assert_eq!(a.len(), 2, "the theme's track, whole, then the arc");
        assert_eq!((a[0].color, a[0].width), (Color::rgb(0.2, 0.2, 0.2), 6.0));
        assert_eq!(
            (a[1].color, a[1].cap),
            (Color::rgb(0.9, 0.1, 0.1), StrokeCap::Round)
        );

        let told = CircularProgressIndicator::determinate(0.5)
            .size(20.0)
            .padding(2.0)
            .stroke_width(2.0)
            .stroke_align(1.0)
            .stroke_cap(StrokeCap::Butt)
            .color(Color::rgb(0.0, 0.0, 1.0));
        assert_eq!(
            Widget::<()>::style_themed(&told, &theme).width,
            Dimension::Length(24.0)
        );
        let a = arcs(&told, &theme, 0.0);
        // Outside: the line's centre is half a stroke out from the 20 px circle.
        assert!(near(a[1].start, on_circle(12.0, 11.0, -PI / 2.0)));
        assert_eq!((a[1].width, a[1].cap), (2.0, StrokeCap::Butt));
        assert_eq!(a[1].color, Color::rgb(0.0, 0.0, 1.0));
    }

    /// **What a reader hears**, as the bar's.
    #[test]
    fn a_reader_hears_the_reference_s_values() {
        let s = Widget::<()>::semantics(
            &CircularProgressIndicator::determinate(0.426).semantics_label("Sync"),
        )
        .unwrap();
        assert_eq!(s.label.as_deref(), Some("Sync"));
        assert_eq!(s.value.as_deref(), Some("43"));
        assert_eq!(
            Widget::<()>::semantics(&CircularProgressIndicator::new())
                .unwrap()
                .value,
            None
        );
    }
}
