//! The parts a [`Slider`](crate::Slider) and a [`RangeSlider`](crate::RangeSlider) are
//! painted with, each replaceable: the track, the tick marks, the thumb, the halo around it
//! and the value indicator over it.
//!
//! A slider asks its own part first, then the theme's ([`SliderTheme`](crate::SliderTheme),
//! which keeps a range slider's track, ticks, thumbs and indicator apart from a slider's), then
//! takes the default. The defaults are the reference's parts, and paint what the sliders always
//! painted.
//!
//! The parts paint; the slider keeps the geometry. The box, the track's inset by the halo's
//! radius and the drag's travel stay the slider's, so a larger custom thumb or halo overflows
//! the box rather than growing it.

use std::fmt;
use std::rc::Rc;

use frus_core::{BorderRadius, Color, Point, Rect, Scene};

use crate::theme::Theme;

/// A slider's colours, resolved for one paint: the widget's, then the theme's, then the
/// reference's — the disabled ones when it is disabled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderPalette {
    /// The travelled part, or a range slider's span between its thumbs.
    pub active_track: Color,
    /// The rest of the track.
    pub inactive_track: Color,
    /// A slider's secondary value, from the thumb towards it.
    pub secondary_track: Color,
    /// The thumb's fill.
    pub thumb: Color,
    /// A ring around the thumb, if one is asked for.
    pub thumb_border: Option<Color>,
    /// A tick mark on the active part.
    pub active_tick_mark: Color,
    /// A tick mark on the rest.
    pub inactive_tick_mark: Color,
}

/// What every part is painted with.
#[derive(Clone, Copy, Debug)]
pub struct SliderPaint<'a> {
    /// The theme in force where the slider is.
    pub theme: &'a Theme,
    /// The slider's colours.
    pub palette: SliderPalette,
    /// Whether the slider can be moved.
    pub enabled: bool,
    /// Whether it has divisions.
    pub discrete: bool,
    /// The opacity everything is painted at.
    pub opacity: f32,
}

impl SliderPaint<'_> {
    /// Whether the travel runs from the right.
    pub fn is_rtl(&self) -> bool {
        self.theme.direction.is_rtl()
    }
}

/// How a thumb is to be painted this time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderThumb {
    /// How far it stands off the track: the resting height, or on its way to the pressed one.
    pub elevation: f32,
    /// On a range slider, whether it lies over the other thumb — the reference rings the one
    /// on top so that the two stay two.
    pub on_top: bool,
}

/// Paints a slider's **track**: the reference's `SliderTrackShape` and
/// `RangeSliderTrackShape` in one.
pub trait SliderTrackShape {
    /// Paints the track along `track` — the travel, inset in the slider's box, as tall as the
    /// theme's track height — with the thumbs' centres across at `thumbs`: one for a slider,
    /// the start and the end for a range slider. `secondary` is where a slider's secondary
    /// value lies, if it has one.
    fn paint(
        &self,
        scene: &mut Scene,
        track: Rect,
        thumbs: &[f32],
        secondary: Option<f32>,
        paint: &SliderPaint<'_>,
    );
}

/// Paints a **tick mark**, one per division: the reference's `SliderTickMarkShape`.
pub trait SliderTickMarkShape {
    /// The mark's width on a track `track_height` tall. The slider leaves the marks out when
    /// three of them would not fit between two, and when this is nothing.
    fn width(&self, track_height: f32) -> f32;

    /// Paints a mark centred on `centre`, on the active part of the track or not.
    fn paint(
        &self,
        scene: &mut Scene,
        centre: Point,
        active: bool,
        track_height: f32,
        paint: &SliderPaint<'_>,
    );
}

/// Paints a **thumb**: the reference's thumb shapes, a slider's and a range slider's.
pub trait SliderThumbShape {
    /// Paints the thumb centred on `centre`.
    fn paint(&self, scene: &mut Scene, centre: Point, thumb: SliderThumb, paint: &SliderPaint<'_>);
}

/// Paints the **halo** around a hovered, focused or held thumb: the reference's overlay
/// shape.
pub trait SliderOverlayShape {
    /// Paints the halo centred on `centre`, grown to `activation` (`0..=1`) in `color`.
    fn paint(
        &self,
        scene: &mut Scene,
        centre: Point,
        activation: f32,
        color: Color,
        paint: &SliderPaint<'_>,
    );
}

/// Paints the **value indicator** over a thumb: the reference's value indicator shapes.
pub trait SliderValueIndicatorShape {
    /// Paints `text` over the thumb centred on `centre`, grown to `scale` (`0..=1`).
    fn paint(
        &self,
        scene: &mut Scene,
        centre: Point,
        text: String,
        scale: f32,
        paint: &SliderPaint<'_>,
    );
}

/// A part held by a slider or a theme: shared, and compared by identity, so that a theme
/// holding one still compares and clones as a value does.
pub struct SliderShape<T: ?Sized>(Rc<T>);

impl<T: ?Sized> Clone for SliderShape<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> PartialEq for SliderShape<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> fmt::Debug for SliderShape<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SliderShape(..)")
    }
}

impl<T: ?Sized> std::ops::Deref for SliderShape<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl SliderShape<dyn SliderTrackShape> {
    /// A track shape to hold.
    pub fn track(shape: impl SliderTrackShape + 'static) -> Self {
        Self(Rc::new(shape))
    }
}

impl SliderShape<dyn SliderTickMarkShape> {
    /// A tick mark shape to hold.
    pub fn tick_mark(shape: impl SliderTickMarkShape + 'static) -> Self {
        Self(Rc::new(shape))
    }
}

impl SliderShape<dyn SliderThumbShape> {
    /// A thumb shape to hold.
    pub fn thumb(shape: impl SliderThumbShape + 'static) -> Self {
        Self(Rc::new(shape))
    }
}

impl SliderShape<dyn SliderOverlayShape> {
    /// A halo shape to hold.
    pub fn overlay(shape: impl SliderOverlayShape + 'static) -> Self {
        Self(Rc::new(shape))
    }
}

impl SliderShape<dyn SliderValueIndicatorShape> {
    /// A value indicator shape to hold.
    pub fn value_indicator(shape: impl SliderValueIndicatorShape + 'static) -> Self {
        Self(Rc::new(shape))
    }
}

/// The part the widget names, else the theme's, else `fallback`.
pub(crate) fn pick<'a, T: ?Sized>(
    widget: &'a Option<SliderShape<T>>,
    theme: &'a Option<SliderShape<T>>,
    fallback: &'a T,
) -> &'a T {
    widget
        .as_ref()
        .or(theme.as_ref())
        .map_or(fallback, |shape| &**shape)
}

/// How much thicker a slider's travelled part, or a range slider's span, is than the rest.
const ACTIVE_EXTRA: f32 = 2.0;

/// A **slider's** default track, the reference's rounded rectangle: the travelled part from
/// the start to the thumb, two pixels thicker and rounded; the rest from the thumb to the end;
/// each reaching half a track past the thumb, under it. A secondary value beyond the thumb
/// runs from it, rounded at its far end.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedRectSliderTrackShape;

impl SliderTrackShape for RoundedRectSliderTrackShape {
    fn paint(
        &self,
        scene: &mut Scene,
        track: Rect,
        thumbs: &[f32],
        secondary: Option<f32>,
        paint: &SliderPaint<'_>,
    ) {
        let track_h = track.height;
        let Some(&thumb) = thumbs.first() else {
            return;
        };
        if track_h <= 0.0 {
            return;
        }
        let o = paint.opacity;
        let rtl = paint.is_rtl();
        let palette = &paint.palette;
        let half = track_h * 0.5;
        let cy = track.y + half;
        let active_half = (track_h + ACTIVE_EXTRA) * 0.5;
        let (left, right) = (track.x, track.x + track.width);
        let (a0, a1) = if rtl {
            (thumb - half, right)
        } else {
            (left, thumb + half)
        };
        let (i0, i1) = if rtl {
            (left, thumb + half)
        } else {
            (thumb - half, right)
        };
        if i1 - i0 > track_h {
            scene.draw_rect(
                Rect::new(i0, cy - half, i1 - i0, track_h),
                palette.inactive_track.fade(o),
                half,
                0.0,
                Color::TRANSPARENT,
            );
        }
        if a1 - a0 > track_h {
            scene.draw_rect(
                Rect::new(a0, cy - active_half, a1 - a0, 2.0 * active_half),
                palette.active_track.fade(o),
                active_half,
                0.0,
                Color::TRANSPARENT,
            );
        }
        // The secondary value: from the thumb towards it, when it lies beyond.
        if let Some(sx) = secondary {
            let beyond = if rtl { sx < thumb } else { sx > thumb };
            if beyond {
                let (s0, s1) = if rtl { (sx, thumb) } else { (thumb, sx) };
                let radius = if rtl {
                    BorderRadius {
                        top_left: half,
                        bottom_left: half,
                        ..BorderRadius::ZERO
                    }
                } else {
                    BorderRadius {
                        top_right: half,
                        bottom_right: half,
                        ..BorderRadius::ZERO
                    }
                };
                scene.draw_rect(
                    Rect::new(s0, cy - half, s1 - s0, track_h),
                    palette.secondary_track.fade(o),
                    radius,
                    0.0,
                    Color::TRANSPARENT,
                );
            }
        }
    }
}

/// A **range slider's** default track, the reference's rounded rectangle: the parts outside
/// the thumbs rounded, the span between them square and two pixels thicker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedRectRangeSliderTrackShape;

impl SliderTrackShape for RoundedRectRangeSliderTrackShape {
    fn paint(
        &self,
        scene: &mut Scene,
        track: Rect,
        thumbs: &[f32],
        _secondary: Option<f32>,
        paint: &SliderPaint<'_>,
    ) {
        let track_h = track.height;
        let (Some(&x0), Some(&x1)) = (thumbs.first(), thumbs.get(1)) else {
            return;
        };
        if track_h <= 0.0 {
            return;
        }
        let o = paint.opacity;
        let palette = &paint.palette;
        let (lo, hi) = (x0.min(x1), x0.max(x1));
        let half = track_h * 0.5;
        let cy = track.y + half;
        let active_half = (track_h + ACTIVE_EXTRA) * 0.5;
        let (left, right) = (track.x, track.x + track.width);
        for (a, b) in [(left, lo + half), (hi - half, right)] {
            if b - a > track_h {
                scene.draw_rect(
                    Rect::new(a, cy - half, b - a, track_h),
                    palette.inactive_track.fade(o),
                    half,
                    0.0,
                    Color::TRANSPARENT,
                );
            }
        }
        if hi - lo > 0.0 {
            scene.draw_rect(
                Rect::new(lo, cy - active_half, hi - lo, 2.0 * active_half),
                palette.active_track.fade(o),
                0.0,
                0.0,
                Color::TRANSPARENT,
            );
        }
    }
}

/// The reference's **round tick mark**: a dot `radius` across from its centre, a quarter of
/// the track's height unless given.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundSliderTickMarkShape {
    /// The dot's radius; a quarter of the track's height when `None`.
    pub radius: Option<f32>,
}

impl RoundSliderTickMarkShape {
    fn radius_on(&self, track_height: f32) -> f32 {
        self.radius.unwrap_or(track_height / 4.0)
    }
}

impl SliderTickMarkShape for RoundSliderTickMarkShape {
    fn width(&self, track_height: f32) -> f32 {
        2.0 * self.radius_on(track_height)
    }

    fn paint(
        &self,
        scene: &mut Scene,
        centre: Point,
        active: bool,
        track_height: f32,
        paint: &SliderPaint<'_>,
    ) {
        let r = self.radius_on(track_height);
        let color = if active {
            paint.palette.active_tick_mark
        } else {
            paint.palette.inactive_tick_mark
        };
        scene.draw_rect(
            Rect::new(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r),
            color.fade(paint.opacity),
            r,
            0.0,
            Color::TRANSPARENT,
        );
    }
}

/// The reference's **round thumb**: a disc `radius` across from its centre over the shadow
/// its elevation casts. A ring two pixels wide when the palette has a thumb border; on a range
/// slider, a one-pixel ring in the surface colour on the thumb on top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundSliderThumbShape {
    /// The disc's radius; the reference's 10.
    pub radius: f32,
}

impl Default for RoundSliderThumbShape {
    fn default() -> Self {
        Self { radius: 10.0 }
    }
}

impl SliderThumbShape for RoundSliderThumbShape {
    fn paint(&self, scene: &mut Scene, centre: Point, thumb: SliderThumb, paint: &SliderPaint<'_>) {
        let o = paint.opacity;
        let r = self.radius;
        let disc = Rect::new(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r);
        if thumb.elevation > 0.0 {
            // The reference's shadows for this height (milestone 606).
            frus_core::paint_elevation(
                scene,
                disc,
                BorderRadius::uniform(r),
                thumb.elevation,
                paint.theme.scheme.shadow.fade(o),
            );
        }
        let (ring_width, ring) = if thumb.on_top {
            (1.0, paint.theme.scheme.surface.fade(o))
        } else if let Some(border) = paint.palette.thumb_border {
            (2.0, border.fade(o))
        } else {
            (0.0, Color::TRANSPARENT)
        };
        scene.draw_rect(disc, paint.palette.thumb.fade(o), r, ring_width, ring);
    }
}

/// The reference's **round halo**: a disc `radius` across at full activation, grown from the
/// thumb's centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundSliderOverlayShape {
    /// The disc's radius at full activation; the reference's 24.
    pub radius: f32,
}

impl Default for RoundSliderOverlayShape {
    fn default() -> Self {
        Self { radius: 24.0 }
    }
}

impl SliderOverlayShape for RoundSliderOverlayShape {
    fn paint(
        &self,
        scene: &mut Scene,
        centre: Point,
        activation: f32,
        color: Color,
        paint: &SliderPaint<'_>,
    ) {
        let activation = activation.clamp(0.0, 1.0);
        if activation <= 0.0 || color.a <= 0.0 {
            return;
        }
        let r = self.radius * activation;
        scene.draw_rect(
            Rect::new(centre.x - r, centre.y - r, 2.0 * r, 2.0 * r),
            color.fade(paint.opacity),
            r,
            0.0,
            Color::TRANSPARENT,
        );
    }
}

/// The reference's **drop**, a slider's default indicator: a rounded label over a point that
/// rests on the thumb.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DropSliderValueIndicatorShape;

impl SliderValueIndicatorShape for DropSliderValueIndicatorShape {
    fn paint(
        &self,
        scene: &mut Scene,
        centre: Point,
        text: String,
        scale: f32,
        paint: &SliderPaint<'_>,
    ) {
        crate::slider::paint_drop(centre, text, paint.theme, scale, paint.opacity, scene);
    }
}

/// The reference's **rectangular** indicator, a range slider's default: a box with corners
/// of 4 over a small triangle whose point is above the thumb.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectangularSliderValueIndicatorShape;

impl SliderValueIndicatorShape for RectangularSliderValueIndicatorShape {
    fn paint(
        &self,
        scene: &mut Scene,
        centre: Point,
        text: String,
        scale: f32,
        paint: &SliderPaint<'_>,
    ) {
        crate::slider::paint_rect_indicator(centre, text, paint.theme, scale, paint.opacity, scene);
    }
}

/// **No part at all**: the reference's `noThumb`, `noOverlay` and `noTickMark`, and a track
/// or an indicator that paints nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NoSliderShape;

impl SliderTrackShape for NoSliderShape {
    fn paint(&self, _: &mut Scene, _: Rect, _: &[f32], _: Option<f32>, _: &SliderPaint<'_>) {}
}

impl SliderTickMarkShape for NoSliderShape {
    fn width(&self, _track_height: f32) -> f32 {
        0.0
    }

    fn paint(&self, _: &mut Scene, _: Point, _: bool, _: f32, _: &SliderPaint<'_>) {}
}

impl SliderThumbShape for NoSliderShape {
    fn paint(&self, _: &mut Scene, _: Point, _: SliderThumb, _: &SliderPaint<'_>) {}
}

impl SliderOverlayShape for NoSliderShape {
    fn paint(&self, _: &mut Scene, _: Point, _: f32, _: Color, _: &SliderPaint<'_>) {}
}

impl SliderValueIndicatorShape for NoSliderShape {
    fn paint(&self, _: &mut Scene, _: Point, _: String, _: f32, _: &SliderPaint<'_>) {}
}

/// The defaults, as statics, for a slider that names no part and a theme that names none.
pub(crate) static SLIDER_TRACK: RoundedRectSliderTrackShape = RoundedRectSliderTrackShape;
pub(crate) static RANGE_TRACK: RoundedRectRangeSliderTrackShape = RoundedRectRangeSliderTrackShape;
pub(crate) static TICK_MARK: RoundSliderTickMarkShape = RoundSliderTickMarkShape { radius: None };
pub(crate) static THUMB: RoundSliderThumbShape = RoundSliderThumbShape { radius: 10.0 };
pub(crate) static OVERLAY: RoundSliderOverlayShape = RoundSliderOverlayShape { radius: 24.0 };
pub(crate) static DROP: DropSliderValueIndicatorShape = DropSliderValueIndicatorShape;
pub(crate) static RECTANGULAR: RectangularSliderValueIndicatorShape =
    RectangularSliderValueIndicatorShape;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::Status;
    use crate::widget::Widget;
    use crate::{RangeSlider, Slider};
    use frus_core::Primitive;
    use std::cell::RefCell;

    /// A box 248 wide: a track of 200 from 24, the halo's radius in.
    const BOX: Rect = Rect::new(0.0, 0.0, 248.0, 48.0);

    /// A part that paints nothing and writes down every call.
    #[derive(Clone, Default)]
    struct Log(Rc<RefCell<Vec<String>>>);

    impl Log {
        fn calls(&self) -> Vec<String> {
            self.0.borrow().clone()
        }

        fn note(&self, line: String) {
            self.0.borrow_mut().push(line);
        }
    }

    impl SliderTrackShape for Log {
        fn paint(
            &self,
            _: &mut Scene,
            track: Rect,
            thumbs: &[f32],
            secondary: Option<f32>,
            _: &SliderPaint<'_>,
        ) {
            self.note(format!("track {track:?} {thumbs:?} {secondary:?}"));
        }
    }

    impl SliderTickMarkShape for Log {
        fn width(&self, _: f32) -> f32 {
            4.0
        }

        fn paint(&self, _: &mut Scene, centre: Point, active: bool, _: f32, _: &SliderPaint<'_>) {
            self.note(format!("tick {} {active}", centre.x));
        }
    }

    impl SliderThumbShape for Log {
        fn paint(&self, _: &mut Scene, centre: Point, thumb: SliderThumb, _: &SliderPaint<'_>) {
            self.note(format!(
                "thumb {} {} {}",
                centre.x, thumb.elevation, thumb.on_top
            ));
        }
    }

    impl SliderOverlayShape for Log {
        fn paint(
            &self,
            _: &mut Scene,
            centre: Point,
            activation: f32,
            _: Color,
            _: &SliderPaint<'_>,
        ) {
            self.note(format!("overlay {} {activation}", centre.x));
        }
    }

    impl SliderValueIndicatorShape for Log {
        fn paint(
            &self,
            _: &mut Scene,
            centre: Point,
            text: String,
            scale: f32,
            _: &SliderPaint<'_>,
        ) {
            self.note(format!("indicator {} {text} {scale}", centre.x));
        }
    }

    fn paint(widget: &dyn Widget<()>, status: Status, theme: &Theme) -> Vec<Primitive> {
        let mut scene = Scene::new();
        widget.paint(BOX, status, theme, &mut scene);
        if widget.paints_over() {
            widget.paint_over(BOX, status, theme, &mut scene);
        }
        scene.primitives().to_vec()
    }

    fn held() -> Status {
        Status {
            interaction: crate::interaction::Interaction::Pressed,
            press_progress: 1.0,
            ..Default::default()
        }
    }

    /// **Each part a slider names paints in its place**: the track given the thumb and the
    /// secondary value, the halo, a tick per division with the side it is on, the thumb with
    /// its height, the value — and the defaults paint nothing more.
    #[test]
    fn a_slider_paints_with_the_parts_it_names() {
        let log = Log::default();
        let slider = Slider::<()>::new(0.5)
            .divisions(2)
            .secondary_track_value(0.75)
            .value_label(|v| format!("{v}"))
            .on_change(|_| ())
            .track_shape(log.clone())
            .tick_mark_shape(log.clone())
            .thumb_shape(log.clone())
            .overlay_shape(log.clone())
            .value_indicator_shape(log.clone());
        let painted = paint(&slider, held(), &Theme::default());
        assert!(painted.is_empty(), "nothing of the defaults: {painted:?}");
        assert_eq!(
            log.calls(),
            vec![
                "track Rect { x: 24.0, y: 22.0, width: 200.0, height: 4.0 } [124.0] Some(173.0)",
                "overlay 124 1",
                "tick 26 true",
                "tick 124 true",
                "tick 222 false",
                "thumb 124 6 false",
                "indicator 124 0.5 1",
            ]
        );
    }

    /// **The theme's parts, a slider's apart from a range slider's**, and the widget's over
    /// the theme's.
    #[test]
    fn the_theme_names_the_parts_for_each_slider() {
        let for_slider = Log::default();
        let for_range = Log::default();
        let mine = Log::default();
        let mut theme = Theme::default();
        let t = &mut theme.widgets.slider;
        t.thumb_shape = Some(SliderShape::thumb(for_slider.clone()));
        t.range_thumb_shape = Some(SliderShape::thumb(for_range.clone()));
        t.track_shape = Some(SliderShape::track(for_slider.clone()));
        t.range_track_shape = Some(SliderShape::track(for_range.clone()));
        t.value_indicator_shape = Some(SliderShape::value_indicator(for_slider.clone()));
        t.range_value_indicator_shape = Some(SliderShape::value_indicator(for_range.clone()));

        paint(&Slider::<()>::new(0.25), Status::default(), &theme);
        assert_eq!(for_slider.calls().len(), 2, "{:?}", for_slider.calls());
        assert!(for_range.calls().is_empty());

        let range = RangeSlider::<()>::new(0.25, 0.75)
            .value_label(|v| format!("{v}"))
            .show_value_indicator(crate::ShowValueIndicator::AlwaysVisible);
        paint(&range, Status::default(), &theme);
        assert_eq!(
            for_range.calls(),
            vec![
                "track Rect { x: 24.0, y: 22.0, width: 200.0, height: 4.0 } [74.0, 174.0] None",
                "thumb 74 1 false",
                "thumb 174 1 false",
                "indicator 74 0.25 1",
                "indicator 174 0.75 1",
            ]
        );
        assert_eq!(for_slider.calls().len(), 2, "not the slider's indicator");

        paint(
            &Slider::<()>::new(0.25).thumb_shape(mine.clone()),
            Status::default(),
            &theme,
        );
        assert_eq!(mine.calls(), vec!["thumb 74 1 false"], "the widget's first");
        assert_eq!(
            for_slider.calls().len(),
            3,
            "only the theme's track this time"
        );
    }

    /// **A range slider's parts**: the track given both thumbs, the thumb on top told so when
    /// they overlap and alone raised, a tick per division with whether it lies between them,
    /// both values, and a thumb stop's halo.
    #[test]
    fn a_range_slider_paints_with_the_parts_it_names() {
        let log = Log::default();
        let range = RangeSlider::<()>::new(0.5, 0.6)
            .divisions(10)
            .value_label(|v| format!("{v}"))
            .on_change(|_, _| ())
            .track_shape(log.clone())
            .tick_mark_shape(log.clone())
            .thumb_shape(log.clone())
            .overlay_shape(log.clone())
            .value_indicator_shape(log.clone());
        let painted = paint(&range, held(), &Theme::default());
        assert!(painted.is_empty(), "nothing of the defaults: {painted:?}");
        let calls = log.calls();
        assert_eq!(
            calls[0],
            "track Rect { x: 24.0, y: 22.0, width: 200.0, height: 4.0 } [124.0, 143.6] None"
        );
        let ticks: Vec<_> = calls.iter().filter(|c| c.starts_with("tick")).collect();
        assert_eq!(ticks.len(), 11);
        assert_eq!(
            ticks.iter().filter(|c| c.ends_with("true")).count(),
            2,
            "{ticks:?}"
        );
        let rest: Vec<_> = calls
            .iter()
            .filter(|c| !c.starts_with("tick") && !c.starts_with("track"))
            .collect();
        assert_eq!(
            rest,
            vec![
                "overlay 143.6 1",
                "thumb 124 1 false",
                "thumb 143.6 6 true",
                "indicator 124 0.5 1",
                "indicator 143.6 0.6 1",
            ]
        );

        // A thumb stop's halo and its value, with the slider's parts.
        let stop = &Widget::<()>::children(&range)[0].children()[0].children()[0];
        let focused = Status {
            focus_progress: 1.0,
            ..Default::default()
        };
        let mut scene = Scene::new();
        stop.paint(
            Rect::new(100.0, 0.0, 48.0, 48.0),
            focused,
            &Theme::default(),
            &mut scene,
        );
        assert!(scene.primitives().is_empty());
        assert_eq!(
            log.calls()[calls.len()..],
            ["overlay 124 1", "indicator 124 0.5 1"]
        );
    }

    /// **No part at all**: no thumb, no halo, no ticks, no track, no value.
    #[test]
    fn no_slider_shape_paints_nothing() {
        let bare = Slider::<()>::new(0.5)
            .divisions(4)
            .value_label(|v| format!("{v}"))
            .track_shape(NoSliderShape)
            .tick_mark_shape(NoSliderShape)
            .thumb_shape(NoSliderShape)
            .overlay_shape(NoSliderShape)
            .value_indicator_shape(NoSliderShape);
        assert!(paint(&bare, held(), &Theme::default()).is_empty());
    }

    /// **The reference's parts, sized**: a larger thumb, halo and tick, and the rectangular
    /// indicator on a slider.
    #[test]
    fn the_reference_parts_take_their_sizes() {
        let theme = Theme::default();
        let crisp = |p: &[Primitive]| -> Vec<Rect> {
            p.iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, blur, .. } if *blur == 0.0 => Some(*rect),
                    _ => None,
                })
                .collect()
        };
        let big = Slider::<()>::new(0.5)
            .on_change(|_| ())
            .thumb_shape(RoundSliderThumbShape { radius: 14.0 })
            .overlay_shape(RoundSliderOverlayShape { radius: 30.0 });
        let rects = crisp(&paint(&big, held(), &theme));
        assert!(
            rects.contains(&Rect::new(110.0, 10.0, 28.0, 28.0)),
            "{rects:?}"
        );
        assert!(
            rects.contains(&Rect::new(94.0, -6.0, 60.0, 60.0)),
            "{rects:?}"
        );

        let dotted = Slider::<()>::new(0.5)
            .divisions(2)
            .tick_mark_shape(RoundSliderTickMarkShape { radius: Some(3.0) });
        let rects = crisp(&paint(&dotted, Status::default(), &theme));
        assert_eq!(
            rects
                .iter()
                .filter(|r| r.width == 6.0 && r.height == 6.0)
                .count(),
            3,
            "{rects:?}"
        );

        // The rectangle over a slider: the drop is a path, the rectangle a rounded box.
        let boxed = Slider::<()>::new(0.5)
            .divisions(2)
            .value_label(|v| format!("{v}"))
            .on_change(|_| ())
            .value_indicator_shape(RectangularSliderValueIndicatorShape);
        let indicator = theme
            .scheme
            .surface
            .fade(0.90)
            .lerp(theme.scheme.on_surface, 0.60);
        let found = paint(&boxed, held(), &theme);
        assert!(found
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == indicator)));
    }

    /// **A held part compares by identity**: a theme holding one is equal to its clone, and
    /// two parts made apart are two.
    #[test]
    fn a_held_part_compares_by_identity() {
        let one = SliderShape::thumb(RoundSliderThumbShape::default());
        assert_eq!(one, one.clone());
        assert_ne!(one, SliderShape::thumb(RoundSliderThumbShape::default()));
        let mut theme = Theme::default();
        theme.widgets.slider.thumb_shape = Some(one);
        assert_eq!(theme.widgets.slider, theme.widgets.slider.clone());
        assert_eq!(
            format!("{:?}", SliderShape::overlay(NoSliderShape)),
            "SliderShape(..)"
        );
    }

    /// **A halo in no colour paints nothing**, rather than an invisible disc.
    #[test]
    fn a_transparent_halo_paints_nothing() {
        let theme = Theme::default();
        let paint = SliderPaint {
            theme: &theme,
            palette: SliderPalette {
                active_track: Color::BLACK,
                inactive_track: Color::BLACK,
                secondary_track: Color::BLACK,
                thumb: Color::BLACK,
                thumb_border: None,
                active_tick_mark: Color::BLACK,
                inactive_tick_mark: Color::BLACK,
            },
            enabled: true,
            discrete: false,
            opacity: 1.0,
        };
        let mut scene = Scene::new();
        let halo = RoundSliderOverlayShape::default();
        halo.paint(
            &mut scene,
            Point::new(50.0, 24.0),
            1.0,
            Color::TRANSPARENT,
            &paint,
        );
        halo.paint(
            &mut scene,
            Point::new(50.0, 24.0),
            0.0,
            Color::BLACK,
            &paint,
        );
        assert!(scene.primitives().is_empty());
        halo.paint(
            &mut scene,
            Point::new(50.0, 24.0),
            0.5,
            Color::BLACK,
            &paint,
        );
        assert_eq!(scene.primitives().len(), 1);
    }

    /// **Ticks that would crowd are left out**, by the width the tick mark gives: three of
    /// them must fit between two.
    #[test]
    fn crowded_ticks_are_left_out() {
        let ticks = |n: usize| {
            let log = Log::default();
            let slider = Slider::<()>::new(0.5)
                .divisions(n)
                .tick_mark_shape(log.clone());
            paint(&slider, Status::default(), &Theme::default());
            log.calls().len()
        };
        // 196 pixels between the first and the last: 16 steps of 12.25 fit, 17 of 11.5 do not.
        assert_eq!(ticks(16), 17);
        assert_eq!(ticks(17), 0);
    }
}
