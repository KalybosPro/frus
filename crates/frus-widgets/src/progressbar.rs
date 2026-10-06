//! [`LinearProgressIndicator`]: a progress bar — **determinate**, filled to a value, or
//! **indeterminate**, two lines running along the track while the length of the work is
//! unknown (`progress_indicator.dart:414`).

use frus_core::{BorderRadius, Color, Curve, Rect, Scene, TextDirection};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// **Bar height**, in logical pixels (`progress_indicator.dart:1624`).
const HEIGHT: f32 = 4.0;
/// The room left between a line and the track, in the 2024 look
/// (`progress_indicator.dart:1636`).
const TRACK_GAP: f32 = 4.0;
/// The radius of the dot at the far end of the track, in the 2024 look
/// (`progress_indicator.dart:1633`).
const STOP_RADIUS: f32 = 2.0;
/// The corners of the bar in the 2024 look: half the **default** height, whatever the
/// height (`progress_indicator.dart:1627`).
const RADIUS_2024: f32 = 2.0;
/// One run of the indeterminate animation, in seconds (`progress_indicator.dart:23`).
const INDETERMINATE_PERIOD: f32 = 1.8;
/// Below this value the gap shrinks with the value, so that it does not appear out of
/// nothing at the first percent (`progress_indicator.dart:30`).
const GAP_RAMP: f32 = 0.01;

/// A progress bar: a track, and either a fill proportional to a value or two lines that
/// run along it while the length of the work is unknown.
///
/// The look is the reference's **default** one, its 2023 appearance: square ends, the fill
/// meeting the track. [`year2023(false)`](Self::year2023) is its newer one — rounded ends,
/// a gap between the fill and the track, and a dot at the far end saying where the bar is
/// headed — which the reference says will become its default in time. Every piece of
/// either is a builder.
pub struct LinearProgressIndicator {
    value: Option<f32>,
    width: Dimension,
    /// Whether the width was given; if not, the bar takes the width on offer, as the
    /// reference's does (milestone 590).
    sized: bool,
    color: Option<Color>,
    track_color: Option<Color>,
    min_height: Option<f32>,
    radius: Option<BorderRadius>,
    stop_indicator_color: Option<Color>,
    stop_indicator_radius: Option<f32>,
    track_gap: Option<f32>,
    year2023: Option<bool>,
    semantics_label: Option<String>,
    semantics_value: Option<String>,
}

impl LinearProgressIndicator {
    /// A **determinate** bar, filled to `value`, clamped to `0.0..=1.0`.
    pub fn new(value: f32) -> Self {
        Self::with_value(Some(value))
    }

    /// An **indeterminate** bar: the work is under way and how long it will take is not
    /// known, so two lines run along the track for as long as it is shown.
    pub fn indeterminate() -> Self {
        Self::with_value(None)
    }

    /// A bar filled to `value`, or indeterminate for `None` — the reference's nullable
    /// `value`.
    pub fn with_value(value: Option<f32>) -> Self {
        Self {
            value: value.map(|v| v.clamp(0.0, 1.0)),
            width: Dimension::Length(200.0),
            sized: false,
            color: None,
            track_color: None,
            min_height: None,
            radius: None,
            stop_indicator_color: None,
            stop_indicator_radius: None,
            track_gap: None,
            year2023: None,
            semantics_label: None,
            semantics_value: None,
        }
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self.sized = true;
        self
    }

    /// **The colour of the fill**, over the theme's and `primary`.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// **The colour of the track** — the reference's `backgroundColor` — over the theme's
    /// and `secondary_container`.
    #[must_use]
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// **How tall the bar is**, over the theme's and the reference's four.
    #[must_use]
    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self
    }

    /// **The corners of the fill, the lines and the track.** Unset, square in the 2023
    /// look and 2 px in the newer one.
    #[must_use]
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// **The dot at the far end of the track**, in the newer look. Unset, the theme's, then
    /// `primary`.
    #[must_use]
    pub fn stop_indicator_color(mut self, color: Color) -> Self {
        self.stop_indicator_color = Some(color);
        self
    }

    /// That dot's radius, in the newer look; never more than half the bar's height.
    /// **Zero draws none.**
    #[must_use]
    pub fn stop_indicator_radius(mut self, radius: f32) -> Self {
        self.stop_indicator_radius = Some(radius);
        self
    }

    /// **The gap between a line and the track**, in the newer look.
    #[must_use]
    pub fn track_gap(mut self, gap: f32) -> Self {
        self.track_gap = Some(gap);
        self
    }

    /// **Which look**: the 2023 one (`true`, the default, as the reference's) or the newer
    /// one (`false`). The stop dot and the track gap exist only in the newer look; in the
    /// 2023 one they are ignored, as the reference ignores them.
    #[must_use]
    pub fn year2023(mut self, year2023: bool) -> Self {
        self.year2023 = Some(year2023);
        self
    }

    /// **What the bar is the progress of**, for whoever cannot see it.
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

    /// How tall the bar is: the caller's word, then the theme's, then the reference's.
    fn height(&self, theme: &Theme) -> f32 {
        self.min_height
            .or(theme.widgets.progress.linear_min_height)
            .unwrap_or(HEIGHT)
    }
}

/// What the painter needs, resolved: the caller's word, then the theme's, then the
/// reference's default for the look in force (`progress_indicator.dart:595`).
struct Resolved {
    value_color: Color,
    track_color: Color,
    radius: Option<BorderRadius>,
    stop: Option<(Color, f32)>,
    gap: f32,
    rtl: bool,
}

impl LinearProgressIndicator {
    fn resolve(&self, theme: &Theme) -> Resolved {
        let t = &theme.widgets.progress;
        let year2023 = self.year2023.or(t.year2023).unwrap_or(true);
        let value_color = self.color.or(t.color).unwrap_or(theme.scheme.primary);
        let newer = |own: Option<f32>, themed: Option<f32>, default: f32| {
            (!year2023).then(|| own.or(themed).unwrap_or(default))
        };
        let stop_radius = newer(
            self.stop_indicator_radius,
            t.stop_indicator_radius,
            STOP_RADIUS,
        );
        Resolved {
            value_color,
            track_color: self
                .track_color
                .or(t.linear_track_color)
                .unwrap_or(theme.scheme.secondary_container),
            radius: self
                .radius
                .or(t.border_radius)
                .or((!year2023).then(|| BorderRadius::uniform(RADIUS_2024))),
            stop: stop_radius.filter(|r| *r > 0.0).map(|r| {
                let color = self
                    .stop_indicator_color
                    .or(t.stop_indicator_color)
                    .unwrap_or(theme.scheme.primary);
                (color, r)
            }),
            gap: newer(self.track_gap, t.track_gap, TRACK_GAP).unwrap_or(0.0),
            rtl: theme.direction == TextDirection::Rtl,
        }
    }
}

/// The four ends of the two indeterminate lines, as the reference times them
/// (`progress_indicator.dart:189`): each an interval of the 1800 ms run on its own cubic.
fn indeterminate_ends(t: f32) -> [f32; 4] {
    let ms = |ms: f32| ms / (INDETERMINATE_PERIOD * 1000.0);
    let curve = |begin: f32, len: f32, x1: f32, y1: f32, x2: f32, y2: f32| Curve::Interval {
        begin: ms(begin),
        end: ms(begin + len),
        inner: Box::new(Curve::Cubic { x1, y1, x2, y2 }),
    };
    [
        curve(0.0, 750.0, 0.2, 0.0, 0.8, 1.0).transform(t),
        curve(333.0, 750.0, 0.4, 0.0, 1.0, 1.0).transform(t),
        curve(1000.0, 567.0, 0.0, 0.0, 0.65, 1.0).transform(t),
        curve(1267.0, 533.0, 0.10, 0.0, 0.45, 1.0).transform(t),
    ]
}

/// The reference's painter (`progress_indicator.dart:211`), on fractions of the bar.
fn paint_bar(
    scene: &mut Scene,
    bounds: Rect,
    value: Option<f32>,
    animation: f32,
    r: &Resolved,
    opacity: f32,
) {
    let draw = |scene: &mut Scene, start: f32, end: f32, color: Color| {
        if end - start <= 0.0 {
            return;
        }
        let (left, right) = if r.rtl {
            (1.0 - end, 1.0 - start)
        } else {
            (start, end)
        };
        scene.draw_rect(
            Rect::new(
                bounds.x + left * bounds.width,
                bounds.y,
                (right - left) * bounds.width,
                bounds.height,
            ),
            color.fade(opacity),
            r.radius.unwrap_or(BorderRadius::uniform(0.0)),
            0.0,
            Color::TRANSPARENT,
        );
    };
    let gap = if bounds.width > 0.0 {
        r.gap / bounds.width
    } else {
        0.0
    };
    // The gap grows from nothing over the first percent rather than appearing whole.
    let ramp = |v: f32| gap * v.clamp(0.0, GAP_RAMP) / GAP_RAMP;

    if let Some(value) = value {
        let track_start = if gap > 0.0 { value + ramp(value) } else { 0.0 };
        if track_start < 1.0 {
            draw(scene, track_start, 1.0, r.track_color);
        }
        if let Some((color, radius)) = r.stop {
            // Never more than half the bar's height, at the far end.
            let max = bounds.height * 0.5;
            let radius = radius.min(max);
            let cx = if r.rtl {
                bounds.x + max
            } else {
                bounds.x + bounds.width - max
            };
            let cy = bounds.y + max;
            scene.draw_rect(
                Rect::new(cx - radius, cy - radius, 2.0 * radius, 2.0 * radius),
                color.fade(opacity),
                radius,
                0.0,
                Color::TRANSPARENT,
            );
        }
        if value > 0.0 {
            draw(scene, 0.0, value, r.value_color);
        }
        return;
    }

    let [head1, tail1, head2, tail2] = indeterminate_ends(animation);
    // The track ahead of line 1.
    if head1 < 1.0 - gap {
        let start = if head1 > 0.0 {
            head1 + ramp(head1)
        } else {
            0.0
        };
        draw(scene, start, 1.0, r.track_color);
    }
    if head1 - tail1 > 0.0 {
        draw(scene, tail1, head1, r.value_color);
    }
    // The track between line 2 and line 1.
    if tail1 > gap {
        let start = if head2 > 0.0 {
            head2 + ramp(head2)
        } else {
            0.0
        };
        let end = if tail1 < 1.0 {
            tail1 - ramp(1.0 - tail1)
        } else {
            1.0
        };
        draw(scene, start, end, r.track_color);
    }
    if head2 - tail2 > 0.0 {
        draw(scene, tail2, head2, r.value_color);
    }
    // The track behind line 2.
    if tail2 > gap {
        let end = if tail2 < 1.0 {
            tail2 - ramp(1.0 - tail2)
        } else {
            1.0
        };
        draw(scene, 0.0, end, r.track_color);
    }
}

impl<Msg> Widget<Msg> for LinearProgressIndicator {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: Dimension::Length(HEIGHT),
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            width: self.width,
            height: Dimension::Length(self.height(theme)),
            ..Default::default()
        }
    }

    /// **All the room on offer** on an axis nobody sized, as the reference's takes
    /// (milestone 590).
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: !self.sized,
            vertical: false,
        }
    }

    /// The default size is only a default: the room replaces it wherever the layout can
    /// give the room, and it stands where the reference would have nothing to give.
    fn soft_extent(&self) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: !self.sized,
            vertical: false,
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    /// An indeterminate bar moves for as long as it is shown; a determinate one only when
    /// its value changes.
    fn continuous(&self) -> bool {
        self.value.is_none()
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let animation = (status.time / INDETERMINATE_PERIOD).rem_euclid(1.0);
        paint_bar(
            scene,
            bounds,
            self.value,
            animation,
            &self.resolve(theme),
            status.opacity,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// A determinate bar says its value from 0 to 100; an indeterminate one says only that
    /// it is busy — a progress bar with no value, which is how the platforms' own
    /// accessibility vocabularies say "indeterminate" (`progress_indicator.dart:146`).
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
    use frus_core::Primitive;

    fn painted_at(
        bar: &LinearProgressIndicator,
        theme: &Theme,
        time: f32,
    ) -> Vec<(Rect, Color, f32)> {
        let mut scene = Scene::new();
        let status = Status {
            time,
            ..Status::default()
        };
        Widget::<()>::paint(
            bar,
            Rect::new(0.0, 0.0, 100.0, HEIGHT),
            status,
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect,
                    color,
                    radius,
                    ..
                } => Some((*rect, *color, radius.top_left)),
                _ => None,
            })
            .collect()
    }

    fn painted(bar: &LinearProgressIndicator, theme: &Theme) -> Vec<(Rect, Color, f32)> {
        painted_at(bar, theme, 0.0)
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    /// **The default look is the reference's default**, its 2023 one: the track under the
    /// whole bar, the fill over it, square ends, no gap and no dot
    /// (`progress_indicator.dart:595`, `:1555`).
    #[test]
    fn the_default_look_is_the_reference_s_default() {
        let theme = Theme::default();
        let half = painted(&LinearProgressIndicator::new(0.5).width(100.0), &theme);
        assert_eq!(half.len(), 2, "a track and a fill: {half:#?}");
        let (track, fill) = (half[0], half[1]);
        assert_eq!(
            (track.0.x, track.0.width),
            (0.0, 100.0),
            "the track is whole"
        );
        assert_eq!(track.1, theme.scheme.secondary_container);
        assert_eq!((fill.0.x, fill.0.width), (0.0, 50.0));
        assert_eq!(fill.1, theme.scheme.primary);
        assert_eq!((track.2, fill.2), (0.0, 0.0), "square ends");
        // A tiny value is drawn tiny: no fill widened to a dot.
        let tiny = painted(&LinearProgressIndicator::new(0.001).width(100.0), &theme);
        assert!(near(tiny[1].0.width, 0.1), "{tiny:#?}");
        // In the 2023 look the newer pieces are ignored, as the reference ignores them.
        let asked = LinearProgressIndicator::new(0.5)
            .width(100.0)
            .track_gap(10.0)
            .stop_indicator_radius(2.0);
        assert_eq!(painted(&asked, &theme).len(), 2);
    }

    /// **The newer look**: a 2 px corner whatever the height, the track a gap after the
    /// fill, the dot at the far end drawn under the fill, and the gap growing from nothing
    /// over the first percent (`progress_indicator.dart:256`, `:1611`).
    #[test]
    fn the_newer_look_is_the_reference_s() {
        let theme = Theme::default();
        let bar = |v: f32| LinearProgressIndicator::new(v).width(100.0).year2023(false);
        let half = painted(&bar(0.5), &theme);
        assert_eq!(half.len(), 3, "track, dot, fill: {half:#?}");
        let (track, dot, fill) = (half[0], half[1], half[2]);
        assert!(near(track.0.x, 50.0 + TRACK_GAP));
        assert_eq!(track.2, RADIUS_2024);
        assert_eq!(
            dot.0,
            Rect::new(96.0, 0.0, 4.0, 4.0),
            "centred at the far end"
        );
        assert_eq!(dot.1, theme.scheme.primary);
        assert_eq!((fill.0.width, fill.2), (50.0, RADIUS_2024));
        // Half a percent: half the gap.
        let ramp = painted(&bar(0.005), &theme);
        assert!(near(ramp[0].0.x, 0.5 + TRACK_GAP * 0.5), "{ramp:#?}");
        // At the end the fill covers the dot; there is no track.
        let full = painted(&bar(1.0), &theme);
        assert_eq!(full.len(), 2, "the dot and the fill over it: {full:#?}");
        assert_eq!(full[1].0.width, 100.0);
        // The dot never outgrows the bar.
        let big = painted(&bar(0.5).stop_indicator_radius(10.0), &theme);
        assert_eq!(big[1].0.width, HEIGHT);
    }

    /// **Right to left, the bar fills from the right** and its dot is at the left
    /// (`progress_indicator.dart:224`).
    #[test]
    fn right_to_left_mirrors_the_bar() {
        let theme = Theme::default().rtl();
        let bar = LinearProgressIndicator::new(0.25)
            .width(100.0)
            .year2023(false);
        let p = painted(&bar, &theme);
        let (track, dot, fill) = (p[0], p[1], p[2]);
        assert_eq!((fill.0.x, fill.0.width), (75.0, 25.0));
        assert!(near(track.0.x, 0.0) && near(track.0.width, 75.0 - TRACK_GAP));
        assert_eq!(dot.0.x, 0.0);
    }

    /// **An indeterminate bar runs two lines along the track** on the reference's timing:
    /// one run is 1.8 s, line 1's head leads its tail, line 2 starts at 1 s, and the bar
    /// keeps the frames coming (`progress_indicator.dart:189`, `:292`).
    #[test]
    fn an_indeterminate_bar_runs_the_reference_s_lines() {
        let theme = Theme::default();
        let bar = LinearProgressIndicator::indeterminate().width(100.0);
        assert!(Widget::<()>::continuous(&bar));
        assert!(!Widget::<()>::continuous(&LinearProgressIndicator::new(
            0.3
        )));
        // At 0.375 s, line 1's head is half way along its own 750 ms cubic, its tail has
        // barely started, and line 2 has not.
        let [h1, t1, h2, t2] = indeterminate_ends(0.375 / INDETERMINATE_PERIOD);
        let cubic = Curve::Cubic {
            x1: 0.2,
            y1: 0.0,
            x2: 0.8,
            y2: 1.0,
        };
        assert!(near(h1, cubic.transform(0.5)));
        assert!(t1 > 0.0 && t1 < h1);
        assert_eq!((h2, t2), (0.0, 0.0));
        let p = painted_at(&bar, &theme, 0.375);
        let lines: Vec<_> = p.iter().filter(|x| x.1 == theme.scheme.primary).collect();
        assert_eq!(lines.len(), 1, "only line 1 so far: {p:#?}");
        assert!(near(lines[0].0.x, t1 * 100.0) && near(lines[0].0.width, (h1 - t1) * 100.0));
        // Between 1 s and 1.083 s both lines are out: line 2 has started, line 1's tail
        // has not reached the end.
        let p = painted_at(&bar, &theme, 1.05);
        assert_eq!(p.iter().filter(|x| x.1 == theme.scheme.primary).count(), 2);
        // And the run repeats.
        assert_eq!(
            painted_at(&bar, &theme, 0.375 + INDETERMINATE_PERIOD),
            painted_at(&bar, &theme, 0.375)
        );
    }

    /// **Everything the bar draws is reachable**, on the usual rungs: the caller, then the
    /// theme, then the reference.
    #[test]
    fn a_bar_answers_to_its_theme_and_to_its_caller() {
        let mut theme = Theme::default();
        theme.widgets.progress.color = Some(Color::rgb(0.1, 0.2, 0.3));
        theme.widgets.progress.linear_track_color = Some(Color::rgb(0.4, 0.5, 0.6));
        theme.widgets.progress.linear_min_height = Some(11.0);
        theme.widgets.progress.year2023 = Some(false);
        theme.widgets.progress.track_gap = Some(0.0);
        theme.widgets.progress.stop_indicator_radius = Some(0.0);

        let bar = LinearProgressIndicator::new(0.5).width(100.0);
        let themed = painted(&bar, &theme);
        assert_eq!(themed.len(), 2, "no gap and no dot: {themed:#?}");
        assert_eq!(themed[0].1, Color::rgb(0.4, 0.5, 0.6));
        assert_eq!(themed[1].1, Color::rgb(0.1, 0.2, 0.3));
        assert_eq!(themed[0].2, RADIUS_2024, "the theme chose the newer look");
        assert_eq!(
            Widget::<()>::style_themed(&bar, &theme).height,
            Dimension::Length(11.0)
        );

        let told = LinearProgressIndicator::new(0.5)
            .width(100.0)
            .color(Color::rgb(0.9, 0.0, 0.0))
            .track_color(Color::rgb(0.0, 0.9, 0.0))
            .track_gap(10.0)
            .min_height(7.0)
            .radius(1.0);
        let painted = painted(&told, &theme);
        assert_eq!(painted[0].1, Color::rgb(0.0, 0.9, 0.0));
        assert_eq!(painted[1].1, Color::rgb(0.9, 0.0, 0.0));
        assert!(near(painted[0].0.x, 60.0));
        assert_eq!(painted[0].2, 1.0);
        assert_eq!(
            Widget::<()>::style_themed(&told, &theme).height,
            Dimension::Length(7.0)
        );
    }

    /// **What a reader hears**: a determinate bar's value from 0 to 100, or the value it
    /// was told to say; an indeterminate one, no value at all (`progress_indicator.dart:146`).
    #[test]
    fn a_reader_hears_the_reference_s_values() {
        let s =
            Widget::<()>::semantics(&LinearProgressIndicator::new(0.426).semantics_label("Upload"))
                .unwrap();
        assert_eq!(s.label.as_deref(), Some("Upload"));
        assert_eq!(s.value.as_deref(), Some("43"));
        let s = Widget::<()>::semantics(&LinearProgressIndicator::indeterminate()).unwrap();
        assert_eq!(s.value, None);
        let s =
            Widget::<()>::semantics(&LinearProgressIndicator::new(0.5).semantics_value("3 of 6"))
                .unwrap();
        assert_eq!(s.value.as_deref(), Some("3 of 6"));
    }

    #[test]
    fn value_is_clamped() {
        let theme = Theme::default();
        let full = painted(&LinearProgressIndicator::new(2.0).width(100.0), &theme);
        assert_eq!(full[1].0.width, 100.0);
        let none = painted(&LinearProgressIndicator::new(-1.0).width(100.0), &theme);
        assert_eq!(none.len(), 1, "only the track");
    }
}
