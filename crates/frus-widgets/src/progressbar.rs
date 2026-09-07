//! [`LinearProgressIndicator`]: a **determinate** progress bar (`0..1`).

use frus_core::{BorderRadius, Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// **Bar height**, in logical pixels (`progress_indicator.dart:1624`). It was eight,
/// which is twice the specification and reads as a slab rather than a line.
const HEIGHT: f32 = 4.0;
/// The room left between the end of the fill and the start of the track
/// (`progress_indicator.dart:1636`).
const TRACK_GAP: f32 = 4.0;
/// The radius of the dot at the far end of the track
/// (`progress_indicator.dart:1633`).
const STOP_RADIUS: f32 = 2.0;

/// A progress bar: a track plus a fill proportional to `value`.
///
/// The look is the reference's **current** one rather than its default one. The
/// reference still defaults to the 2023 appearance behind a `year2023` flag it has
/// already deprecated — square ends, no gap, no stop dot — and says in as many
/// words that the flag will default to the newer look in time. A framework with no
/// installed base to keep faith with should start where that one is going: a gap between
/// the fill and the track, and a dot at the end saying where the bar is headed. Every
/// piece of it is a builder, so an application that wants the older look can write it.
pub struct LinearProgressIndicator {
    value: f32,
    width: Dimension,
    color: Option<Color>,
    track_color: Option<Color>,
    min_height: Option<f32>,
    radius: Option<BorderRadius>,
    stop_indicator_color: Option<Color>,
    stop_indicator_radius: Option<f32>,
    track_gap: Option<f32>,
}

impl LinearProgressIndicator {
    /// Creates a bar filled to `value`, clamped to `0.0..=1.0`.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            width: Dimension::Length(200.0),
            color: None,
            track_color: None,
            min_height: None,
            radius: None,
            stop_indicator_color: None,
            stop_indicator_radius: None,
            track_gap: None,
        }
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// **The colour of the fill**, over the theme's and `primary`.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// **The colour of the track**, over the theme's and `secondary_container`.
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

    /// **The corners of the bar and its track.** Unset, fully rounded;
    /// `radius(0.0)` is the square-ended bar the reference still draws by default.
    #[must_use]
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// **The dot at the far end of the track**, which says where the bar is going.
    #[must_use]
    pub fn stop_indicator_color(mut self, color: Color) -> Self {
        self.stop_indicator_color = Some(color);
        self
    }

    /// That dot's radius. **Zero draws none**, which is the older look.
    #[must_use]
    pub fn stop_indicator_radius(mut self, radius: f32) -> Self {
        self.stop_indicator_radius = Some(radius);
        self
    }

    /// **The gap between the fill and the track.** Zero is the older look, where the two
    /// meet.
    #[must_use]
    pub fn track_gap(mut self, gap: f32) -> Self {
        self.track_gap = Some(gap);
        self
    }

    /// How tall the bar is: the caller's word, then the theme's, then the reference's.
    fn height(&self, theme: &Theme) -> f32 {
        self.min_height
            .or(theme.widgets.progress.linear_min_height)
            .unwrap_or(HEIGHT)
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

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let t = &theme.widgets.progress;
        let radius = self
            .radius
            .or(t.border_radius)
            .unwrap_or(BorderRadius::uniform(bounds.height * 0.5));
        let fill_color = self.color.or(t.color).unwrap_or(theme.primary);
        // `secondary_container`, not a faded `muted`. A track drawn as thirty percent of
        // something else is a colour nobody chose and no theme could name; the reference
        // gives it a role (`progress_indicator.dart:1621`).
        let track_color = self
            .track_color
            .or(t.linear_track_color)
            .unwrap_or(theme.scheme.secondary_container);
        let gap = self.track_gap.or(t.track_gap).unwrap_or(TRACK_GAP);
        let stop_radius = self
            .stop_indicator_radius
            .or(t.stop_indicator_radius)
            .unwrap_or(STOP_RADIUS);
        let stop_color = self
            .stop_indicator_color
            .or(t.stop_indicator_color)
            .unwrap_or(fill_color);

        // The fill: at least a dot when the value is above zero, never wider than the
        // bar. A bar at 0.1 % that draws nothing tells the reader the work has not
        // started.
        let fill_w = match bounds.width * self.value {
            w if w > 0.0 => w.max(bounds.height).min(bounds.width),
            _ => 0.0,
        };

        // The track starts **after** the gap, so the two do not meet. When the fill has
        // reached the end there is no track left and none is drawn.
        let track_x = match fill_w {
            0.0 => bounds.x,
            w => bounds.x + w + gap,
        };
        let track_w = (bounds.x + bounds.width - track_x).max(0.0);
        if track_w > 0.0 {
            scene.draw_rect(
                Rect::new(track_x, bounds.y, track_w, bounds.height),
                track_color.fade(o),
                radius,
                0.0,
                Color::TRANSPARENT,
            );
        }
        if fill_w > 0.0 {
            scene.draw_rect(
                Rect::new(bounds.x, bounds.y, fill_w, bounds.height),
                fill_color.fade(o),
                radius,
                0.0,
                Color::TRANSPARENT,
            );
        }
        // The stop: a dot at the far end of the track, saying where the bar is going. It
        // is drawn only while there is track wide enough to hold it — at the end of a
        // run the fill has taken its place.
        if stop_radius > 0.0 && track_w >= 2.0 * stop_radius {
            let cx = bounds.x + bounds.width - bounds.height * 0.5;
            let cy = bounds.y + bounds.height * 0.5;
            scene.draw_rect(
                Rect::new(
                    cx - stop_radius,
                    cy - stop_radius,
                    2.0 * stop_radius,
                    2.0 * stop_radius,
                ),
                stop_color.fade(o),
                stop_radius,
                0.0,
                Color::TRANSPARENT,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let pct = (self.value * 100.0).round();
        Some(
            frus_core::SemanticsProperties::new(frus_core::Role::ProgressBar)
                .value(format!("{pct}%"))
                .range(0.0, self.value, 1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn painted(bar: &LinearProgressIndicator, theme: &Theme) -> Vec<(Rect, frus_core::Color)> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            bar,
            Rect::new(0.0, 0.0, 100.0, HEIGHT),
            Status::default(),
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    fn fill_and_track(value: f32) -> (f32, f32) {
        let bar = LinearProgressIndicator::new(value).width(100.0);
        let widths: Vec<f32> = painted(&bar, &Theme::default())
            .into_iter()
            .map(|(rect, _)| rect.width)
            .collect();
        // [track, fill]
        (widths[0], *widths.get(1).unwrap_or(&0.0))
    }

    /// **A bar was drawn to no specification.** Its height was eight, twice the
    /// reference's four (`progress_indicator.dart:1624`), and its track was `muted` at
    /// thirty percent — a colour nobody chose and no theme could name, where the
    /// reference gives it the `secondary_container` role
    /// (`progress_indicator.dart:1621`).
    #[test]
    fn a_bar_is_drawn_to_a_specification() {
        assert_eq!(HEIGHT, 4.0, "the reference's height, not twice it");
        let theme = Theme::default();
        let bar = LinearProgressIndicator::new(0.5).width(100.0);
        let painted = painted(&bar, &theme);
        assert_eq!(
            painted[0].1, theme.scheme.secondary_container,
            "the track has a role"
        );
        assert_eq!(painted[1].1, theme.primary, "and the fill is the accent");
    }

    /// **The track stops short of the fill, and ends in a dot.** Both are the
    /// reference's current appearance (`progress_indicator.dart:1633` and `:1636`), the
    /// one it says in as many words it is moving to; the gap is what keeps a bar from
    /// reading as one solid rule, and the dot says where it is going.
    ///
    /// At the end of a run there is neither: the fill has taken the whole width, so
    /// there is no track to leave a gap in and nowhere to put the dot.
    #[test]
    fn a_track_leaves_room_for_the_fill_and_a_dot() {
        let theme = Theme::default();
        let half = painted(&LinearProgressIndicator::new(0.5).width(100.0), &theme);
        assert_eq!(half.len(), 3, "a track, a fill and a dot: {half:#?}");
        assert_eq!(half[1].0.width, 50.0, "the fill is half the bar");
        assert_eq!(
            half[0].0.x,
            50.0 + TRACK_GAP,
            "and the track starts a gap after it"
        );
        assert_eq!(half[0].0.width, 100.0 - 50.0 - TRACK_GAP);
        assert_eq!(
            half[2].0.width,
            2.0 * STOP_RADIUS,
            "the dot at the far end: {half:#?}"
        );
        assert!(half[2].0.x + half[2].0.width <= 100.0, "and inside the bar");

        let full = painted(&LinearProgressIndicator::new(1.0).width(100.0), &theme);
        assert_eq!(
            full.len(),
            1,
            "at the end there is only the fill: {full:#?}"
        );
        assert_eq!(full[0].0.width, 100.0);
    }

    /// **Everything the bar draws is now reachable**, on the usual rungs. Not one of
    /// these was: the widget had a single builder, a width, and decided the rest inside
    /// `paint`.
    #[test]
    fn a_bar_answers_to_its_theme_and_to_its_caller() {
        let mut theme = Theme::default();
        theme.widgets.progress.color = Some(frus_core::Color::rgb(0.1, 0.2, 0.3));
        theme.widgets.progress.linear_track_color = Some(frus_core::Color::rgb(0.4, 0.5, 0.6));
        theme.widgets.progress.track_gap = Some(0.0);
        theme.widgets.progress.stop_indicator_radius = Some(0.0);
        theme.widgets.progress.linear_min_height = Some(11.0);

        let bar = LinearProgressIndicator::new(0.5).width(100.0);
        let themed = painted(&bar, &theme);
        assert_eq!(themed.len(), 2, "no gap and no dot is the older look");
        assert_eq!(themed[0].1, frus_core::Color::rgb(0.4, 0.5, 0.6));
        assert_eq!(themed[1].1, frus_core::Color::rgb(0.1, 0.2, 0.3));
        assert_eq!(themed[0].0.x, 50.0, "the track meets the fill");
        assert_eq!(
            Widget::<()>::style_themed(&bar, &theme).height,
            frus_layout::Dimension::Length(11.0),
            "and the box is the theme's height"
        );

        // The caller outranks all of it.
        let told = LinearProgressIndicator::new(0.5)
            .width(100.0)
            .color(frus_core::Color::rgb(0.9, 0.0, 0.0))
            .track_color(frus_core::Color::rgb(0.0, 0.9, 0.0))
            .track_gap(10.0)
            .min_height(7.0);
        let painted = painted(&told, &theme);
        assert_eq!(painted[0].1, frus_core::Color::rgb(0.0, 0.9, 0.0));
        assert_eq!(painted[1].1, frus_core::Color::rgb(0.9, 0.0, 0.0));
        assert_eq!(painted[0].0.x, 60.0);
        assert_eq!(
            Widget::<()>::style_themed(&told, &theme).height,
            frus_layout::Dimension::Length(7.0)
        );
    }

    #[test]
    fn fill_is_proportional_to_value() {
        let (track, fill) = fill_and_track(0.5);
        // The track is what is **left**, now that it stops short of the fill.
        assert_eq!(track, 100.0 - 50.0 - TRACK_GAP);
        assert_eq!(fill, 50.0);
    }

    #[test]
    fn value_is_clamped() {
        // value > 1 → a full fill (= the bar's width), no more. There is no track left
        // to draw at that point, so the fill is the only thing in the frame.
        let full = painted(
            &LinearProgressIndicator::new(2.0).width(100.0),
            &Theme::default(),
        );
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].0.width, 100.0);
        // value 0 → no fill primitive at all.
        let none = painted(
            &LinearProgressIndicator::new(0.0).width(100.0),
            &Theme::default(),
        );
        assert_eq!(none.len(), 2, "the whole track, and the dot at its end");
        assert_eq!(none[0].0.width, 100.0);
    }
}
