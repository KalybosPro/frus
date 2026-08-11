//! The **overscroll glow**: the arc of light that swells at the edge of a
//! scrollable when the content will not go any further.
//!
//! A platform that bounces answers "there is nothing more" with movement — the
//! content pulls away and springs back, and the user needs no other signal. A
//! platform that clamps refuses the movement outright, so without something else
//! the edge is simply **dead**: the finger drags and nothing at all happens, which
//! reads as a frozen app rather than as the end of a list.
//!
//! The glow is that something else. It is the counterpart of the bounce, and the
//! reason [`ScrollPhysics::Clamping`](crate::physics::ScrollPhysics) was only half a
//! behaviour until now.
//!
//! It answers two different events, and they look different on purpose:
//!
//! - [`OverscrollGlow::absorb_impact`] — a **fling** hit the edge. A quick, bright
//!   flash whose strength comes from the speed that was absorbed: the faster the
//!   content was going, the harder it appears to have landed.
//! - [`OverscrollGlow::pull`] — a **finger** is dragging past the edge. The glow
//!   grows with the accumulated pull, holds while the finger stays, and decays
//!   slowly if it lingers.
//!
//! Both then recede. Everything is a pure function of elapsed time — the shell
//! feeds events in and calls [`OverscrollGlow::advance`] once a frame — so the whole
//! state machine is testable with no window and no GPU.

use frus_core::{Color, Curve, Path, Point, Rect, Scene};

/// Which edge of a viewport a glow sits on.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GlowEdge {
    Top,
    Bottom,
    Left,
    Right,
}

impl GlowEdge {
    /// The four edges, in the order [`ScrollGlows`] stores them.
    pub const ALL: [GlowEdge; 4] = [
        GlowEdge::Top,
        GlowEdge::Bottom,
        GlowEdge::Left,
        GlowEdge::Right,
    ];

    fn index(self) -> usize {
        match self {
            GlowEdge::Top => 0,
            GlowEdge::Bottom => 1,
            GlowEdge::Left => 2,
            GlowEdge::Right => 3,
        }
    }

    /// Does this edge sit on the vertical axis?
    fn vertical(self) -> bool {
        matches!(self, GlowEdge::Top | GlowEdge::Bottom)
    }
}

/// What the glow is currently doing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GlowState {
    /// Nothing to draw.
    Idle,
    /// Flashing after a fling hit the edge.
    Absorb,
    /// Growing under a finger that is still pulling.
    Pull,
    /// Fading away.
    Recede,
}

/// How long a glow takes to fade once the gesture is over.
const RECEDE_TIME: f32 = 0.600;
/// How long a pull takes to reach its new size.
const PULL_TIME: f32 = 0.167;
/// How long a pull is held before it starts decaying on its own.
const PULL_HOLD_TIME: f32 = 0.167;
/// How long that unattended decay takes — much slower than letting go.
const PULL_DECAY_TIME: f32 = 2.000;
/// The half-life of the sideways slide towards the finger, in seconds.
const CROSS_AXIS_HALF_TIME: f32 = 1.0 / 60.0;

/// The glow never gets more opaque than this.
const MAX_OPACITY: f32 = 0.5;
/// How much of a pull, relative to the viewport, turns into opacity.
const PULL_OPACITY_GLOW_FACTOR: f32 = 0.8;
/// How much of an absorbed speed turns into opacity.
const VELOCITY_GLOW_FACTOR: f32 = 0.000_06;
/// The arc's height as a fraction of the edge's length: `¾·(2 − √3)`.
const WIDTH_TO_HEIGHT_FACTOR: f32 = 0.75 * (2.0 - 1.732_050_8);
/// Absorbed speeds are clamped into this range, in px/s.
const MIN_ABSORB_VELOCITY: f32 = 100.0;
const MAX_ABSORB_VELOCITY: f32 = 10_000.0;

/// The glow on one edge of one scrollable.
///
/// Two animated quantities — opacity and size — each interpolated from where it
/// was to where the last event asked it to go, shaped by a decelerating curve. That
/// "from where it was" is what keeps a second event mid-flight from making the glow
/// jump.
#[derive(Copy, Clone, Debug)]
pub struct OverscrollGlow {
    state: GlowState,
    /// Progress of the current transition, in seconds, and how long it lasts.
    elapsed: f32,
    duration: f32,
    opacity_from: f32,
    opacity_to: f32,
    size_from: f32,
    size_to: f32,
    /// Where along the edge the arc is centred, in `0..=1`, and where it is
    /// heading — it slides towards the finger rather than jumping to it.
    displacement: f32,
    displacement_target: f32,
    /// The accumulated pull, which is what makes a long drag glow more than a
    /// short one even though each frame's overscroll is small.
    pull_distance: f32,
    /// Time left before an unattended pull starts to decay; `None` when nothing is
    /// being held.
    hold_left: Option<f32>,
}

impl Default for OverscrollGlow {
    fn default() -> Self {
        Self {
            state: GlowState::Idle,
            elapsed: 0.0,
            duration: 0.0,
            opacity_from: 0.0,
            opacity_to: 0.0,
            size_from: 0.0,
            size_to: 0.0,
            displacement: 0.5,
            displacement_target: 0.5,
            pull_distance: 0.0,
            hold_left: None,
        }
    }
}

impl OverscrollGlow {
    /// Progress through the current transition, shaped by the decelerating curve.
    fn t(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        Curve::Decelerate.transform((self.elapsed / self.duration).clamp(0.0, 1.0))
    }

    /// The opacity to paint with, `0` when there is nothing to show.
    pub fn opacity(&self) -> f32 {
        self.opacity_from + (self.opacity_to - self.opacity_from) * self.t()
    }

    /// The size factor, `0..=1`, scaling the arc's height.
    pub fn size(&self) -> f32 {
        self.size_from + (self.size_to - self.size_from) * self.t()
    }

    /// Is there nothing left to draw or animate?
    pub fn is_idle(&self) -> bool {
        self.state == GlowState::Idle
    }

    /// A fling hit this edge at `velocity` px/s (a magnitude, so positive).
    ///
    /// The flash is brief and its strength comes from the speed: this is the visual
    /// consequence of the momentum the clamping physics just threw away.
    pub fn absorb_impact(&mut self, velocity: f32) {
        let velocity = velocity
            .abs()
            .clamp(MIN_ABSORB_VELOCITY, MAX_ABSORB_VELOCITY);
        self.hold_left = None;
        // A glow already on screen grows from where it is; a cold start begins part
        // way up, so that even a gentle landing registers.
        self.opacity_from = if self.state == GlowState::Idle {
            0.3
        } else {
            self.opacity()
        };
        self.opacity_to = (velocity * VELOCITY_GLOW_FACTOR).clamp(self.opacity_from, MAX_OPACITY);
        self.size_from = self.size();
        self.size_to = (0.025 + 7.5e-7 * velocity * velocity).min(1.0);
        // Milliseconds, and deliberately short: an impact is a flash, not a swell.
        self.duration = (0.15 + velocity * 0.02) / 1000.0;
        self.elapsed = 0.0;
        self.displacement = 0.5;
        self.state = GlowState::Absorb;
    }

    /// A finger is dragging `overscroll` px past this edge.
    ///
    /// `extent` is the viewport along the scroll axis, `cross_offset` where the
    /// finger is along the edge and `cross_extent` the edge's length — the last two
    /// are what make the arc appear under the thumb instead of always in the middle.
    pub fn pull(&mut self, overscroll: f32, extent: f32, cross_offset: f32, cross_extent: f32) {
        let extent = extent.max(1.0);
        let cross_extent = cross_extent.max(1.0);
        self.hold_left = Some(PULL_HOLD_TIME);
        // The divisor is empirical: it is what makes the growth match the platform's.
        self.pull_distance += overscroll.abs() / 200.0;
        self.opacity_from = self.opacity();
        self.opacity_to = (self.opacity_from + overscroll.abs() / extent * PULL_OPACITY_GLOW_FACTOR)
            .min(MAX_OPACITY);
        let height = extent.min(cross_extent * WIDTH_TO_HEIGHT_FACTOR);
        self.size_from = self.size();
        // Never smaller than it already is: a pull only ever swells.
        self.size_to =
            (1.0 - 1.0 / (0.7 * (self.pull_distance * height).sqrt())).max(self.size_from);
        self.displacement_target = (cross_offset / cross_extent).clamp(0.0, 1.0);
        self.duration = PULL_TIME;
        if self.state != GlowState::Pull {
            self.elapsed = 0.0;
            self.state = GlowState::Pull;
        }
    }

    /// The gesture ended: a pull now fades at the "let go" rate.
    pub fn scroll_end(&mut self) {
        if self.state == GlowState::Pull {
            self.recede(RECEDE_TIME);
        }
    }

    /// Starts the fade.
    fn recede(&mut self, duration: f32) {
        if matches!(self.state, GlowState::Recede | GlowState::Idle) {
            return;
        }
        self.hold_left = None;
        self.opacity_from = self.opacity();
        self.opacity_to = 0.0;
        self.size_from = self.size();
        self.size_to = 0.0;
        self.duration = duration;
        self.elapsed = 0.0;
        self.state = GlowState::Recede;
    }

    /// Advances by `dt` seconds. Returns `true` while there is still something to
    /// animate.
    pub fn advance(&mut self, dt: f32) -> bool {
        if self.state == GlowState::Idle {
            return false;
        }
        self.elapsed += dt;
        // The arc slides towards the finger by halving the remaining distance every
        // `CROSS_AXIS_HALF_TIME` — frame-rate independent, unlike a fixed step.
        if (self.displacement_target - self.displacement).abs() > 1e-3 {
            let decay = 2f32.powf(-dt / CROSS_AXIS_HALF_TIME);
            self.displacement =
                self.displacement_target - (self.displacement_target - self.displacement) * decay;
        } else {
            self.displacement = self.displacement_target;
        }
        // A pull left unattended starts decaying on its own, slowly.
        if let Some(left) = self.hold_left.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.hold_left = None;
                self.recede(PULL_DECAY_TIME);
            }
        }
        if self.elapsed >= self.duration {
            match self.state {
                // A flash always fades straight afterwards.
                GlowState::Absorb => self.recede(RECEDE_TIME),
                GlowState::Recede => {
                    self.state = GlowState::Idle;
                    self.pull_distance = 0.0;
                    self.opacity_from = 0.0;
                    self.opacity_to = 0.0;
                    self.size_from = 0.0;
                    self.size_to = 0.0;
                    return false;
                }
                // A pull holds at its size until the finger says otherwise.
                GlowState::Pull | GlowState::Idle => {}
            }
        }
        true
    }

    /// Paints the arc on `edge` of `viewport`.
    ///
    /// The shape is a wide, shallow ellipse whose visible sliver is the part inside
    /// a thin band along the edge — the same construction on all four sides, with
    /// the band's frame swapped and mirrored to suit.
    pub fn paint(&self, viewport: Rect, edge: GlowEdge, color: Color, scene: &mut Scene) {
        let opacity = self.opacity();
        if opacity <= 0.001 || self.size() <= 0.0 {
            return;
        }
        // In the edge's own frame, `along` runs the length of the edge and `into`
        // points away from it, into the content.
        let (along, into) = if edge.vertical() {
            (viewport.width, viewport.height)
        } else {
            (viewport.height, viewport.width)
        };
        if along <= 0.0 || into <= 0.0 {
            return;
        }

        // A viewport wider than it is deep would give an absurdly tall arc; the base
        // scale keeps the glow proportional to the shallower dimension.
        let base_scale = if along > into { into / along } else { 1.0 };
        let radius = along * 1.5;
        let band = into.min(along * WIDTH_TO_HEIGHT_FACTOR);
        let scale_y = self.size() * base_scale;
        let center_along = (along / 2.0) * (0.5 + self.displacement);
        // The circle sits mostly outside the viewport: only its cap shows. Scaling
        // it towards the edge is what turns the cap into a flat, wide arc.
        let center_into = (band - radius) * scale_y;
        let oval = Rect::new(
            center_along - radius,
            center_into - radius * scale_y,
            radius * 2.0,
            radius * 2.0 * scale_y,
        );

        // Map the edge's frame onto the screen. Each case is a mirror, a transpose
        // or both, so an axis-aligned ellipse stays one.
        let map = |r: Rect| -> Rect {
            match edge {
                GlowEdge::Top => Rect::new(viewport.x + r.x, viewport.y + r.y, r.width, r.height),
                GlowEdge::Bottom => Rect::new(
                    viewport.x + r.x,
                    viewport.y + viewport.height - r.y - r.height,
                    r.width,
                    r.height,
                ),
                GlowEdge::Left => Rect::new(viewport.x + r.y, viewport.y + r.x, r.height, r.width),
                GlowEdge::Right => Rect::new(
                    viewport.x + viewport.width - r.y - r.height,
                    viewport.y + r.x,
                    r.height,
                    r.width,
                ),
            }
        };

        let band_rect = map(Rect::new(0.0, 0.0, along, band));
        let previous = scene.current_clip();
        scene.set_clip(previous.intersect(band_rect));
        scene.fill_path(&Path::oval(map(oval)), color.fade(opacity));
        scene.set_clip(previous);
    }
}

/// The four glows of one scrollable region.
///
/// All four are always present and cost nothing while idle, which spares every
/// caller an `Option` and a lookup for the common case of a list that is simply
/// being scrolled.
#[derive(Copy, Clone, Debug, Default)]
pub struct ScrollGlows([OverscrollGlow; 4]);

impl ScrollGlows {
    /// The glow on `edge`.
    pub fn edge(&self, edge: GlowEdge) -> &OverscrollGlow {
        &self.0[edge.index()]
    }

    /// The glow on `edge`, to drive.
    pub fn edge_mut(&mut self, edge: GlowEdge) -> &mut OverscrollGlow {
        &mut self.0[edge.index()]
    }

    /// Is every edge quiet?
    pub fn is_idle(&self) -> bool {
        self.0.iter().all(OverscrollGlow::is_idle)
    }

    /// Advances all four. Returns `true` while any is still animating.
    pub fn advance(&mut self, dt: f32) -> bool {
        let mut animating = false;
        for glow in &mut self.0 {
            animating |= glow.advance(dt);
        }
        animating
    }

    /// Tells every edge the gesture is over.
    pub fn scroll_end(&mut self) {
        for glow in &mut self.0 {
            glow.scroll_end();
        }
    }

    /// Paints all four onto `viewport`.
    pub fn paint(&self, viewport: Rect, color: Color, scene: &mut Scene) {
        for edge in GlowEdge::ALL {
            self.edge(edge).paint(viewport, edge, color, scene);
        }
    }
}

/// The edge a scroll offset ran past, if it did: `delta` is the refused movement,
/// negative meaning the content was pulled towards the start.
pub fn edge_for(vertical: bool, delta: f32) -> GlowEdge {
    match (vertical, delta < 0.0) {
        (true, true) => GlowEdge::Top,
        (true, false) => GlowEdge::Bottom,
        (false, true) => GlowEdge::Left,
        (false, false) => GlowEdge::Right,
    }
}

/// Where a glow's arc should be centred along `edge`, given a pointer at `point`
/// inside `viewport` — and how long that edge is.
pub fn cross_axis(viewport: Rect, edge: GlowEdge, point: Point) -> (f32, f32) {
    if edge.vertical() {
        ((point.x - viewport.x).clamp(0.0, viewport.width), viewport.width)
    } else {
        ((point.y - viewport.y).clamp(0.0, viewport.height), viewport.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_glow_shows_nothing() {
        let glow = OverscrollGlow::default();
        assert!(glow.is_idle());
        assert_eq!(glow.opacity(), 0.0);
        let mut glow = glow;
        assert!(!glow.advance(0.016), "an idle glow does not animate");
    }

    #[test]
    fn an_impact_flashes_then_fades_to_nothing() {
        let mut glow = OverscrollGlow::default();
        glow.absorb_impact(3000.0);
        assert!(!glow.is_idle());
        assert!(glow.opacity() > 0.0, "visible at once");
        // It runs, and it ends: no glow outlives its gesture.
        let mut frames = 0;
        while glow.advance(1.0 / 60.0) {
            frames += 1;
            assert!(frames < 300, "the flash never finished");
        }
        assert!(glow.is_idle());
        assert_eq!(glow.opacity(), 0.0);
    }

    #[test]
    fn a_faster_impact_glows_harder() {
        let peak = |velocity: f32| {
            let mut glow = OverscrollGlow::default();
            glow.absorb_impact(velocity);
            let mut peak: f32 = 0.0;
            loop {
                peak = peak.max(glow.opacity() * glow.size());
                if !glow.advance(1.0 / 60.0) {
                    break;
                }
            }
            peak
        };
        assert!(
            peak(6000.0) > peak(600.0),
            "the harder landing should show more"
        );
    }

    #[test]
    fn the_absorbed_opacity_is_capped() {
        let mut glow = OverscrollGlow::default();
        glow.absorb_impact(1e9); // absurd, and clamped
        let mut peak: f32 = 0.0;
        loop {
            peak = peak.max(glow.opacity());
            if !glow.advance(1.0 / 60.0) {
                break;
            }
        }
        assert!(peak <= MAX_OPACITY + 1e-4, "peak = {peak}");
    }

    #[test]
    fn a_longer_pull_glows_more_than_a_short_one() {
        let after = |pulls: usize| {
            let mut glow = OverscrollGlow::default();
            for _ in 0..pulls {
                glow.pull(12.0, 800.0, 200.0, 400.0);
                glow.advance(1.0 / 60.0);
            }
            glow.size()
        };
        assert!(after(20) > after(2), "the pull must accumulate");
    }

    #[test]
    fn letting_go_of_a_pull_fades_it() {
        let mut glow = OverscrollGlow::default();
        glow.pull(40.0, 800.0, 200.0, 400.0);
        glow.advance(1.0 / 60.0);
        assert!(glow.opacity() > 0.0);
        glow.scroll_end();
        let mut frames = 0;
        while glow.advance(1.0 / 60.0) {
            frames += 1;
            assert!(frames < 200, "the fade never finished");
        }
        assert!(glow.is_idle());
    }

    #[test]
    fn a_pull_left_alone_decays_on_its_own() {
        // The finger stopped moving but never lifted: the glow must not sit there
        // for ever.
        let mut glow = OverscrollGlow::default();
        glow.pull(40.0, 800.0, 200.0, 400.0);
        // Past the hold time, with no further pull and no release.
        for _ in 0..20 {
            glow.advance(1.0 / 60.0);
        }
        let held = glow.opacity();
        for _ in 0..60 {
            glow.advance(1.0 / 60.0);
        }
        assert!(glow.opacity() < held, "it should be fading by itself");
    }

    #[test]
    fn the_arc_slides_towards_the_finger() {
        let mut glow = OverscrollGlow::default();
        // A pull at the far right of a 400 px wide edge.
        glow.pull(20.0, 800.0, 400.0, 400.0);
        let start = glow.displacement;
        assert_eq!(start, 0.5, "it starts centred");
        for _ in 0..10 {
            glow.advance(1.0 / 60.0);
        }
        assert!(
            glow.displacement > start && glow.displacement <= 1.0,
            "displacement = {}",
            glow.displacement
        );
    }

    #[test]
    fn a_glow_paints_inside_its_edge_band() {
        let viewport = Rect::new(10.0, 20.0, 400.0, 600.0);
        for edge in GlowEdge::ALL {
            let mut glow = OverscrollGlow::default();
            glow.absorb_impact(4000.0);
            glow.advance(0.05);
            let mut scene = Scene::new();
            scene.set_clip(Rect::new(0.0, 0.0, 1000.0, 1000.0));
            glow.paint(viewport, edge, Color::rgb(0.0, 1.0, 0.0), &mut scene);
            assert_eq!(scene.len(), 1, "{edge:?} should paint exactly one shape");
            let bounds = scene.primitives()[0].bounds();
            // Whatever the ellipse's own size, the clip keeps it against its edge.
            let clip = match scene.primitives()[0] {
                frus_core::Primitive::Path { clip, .. } => clip,
                _ => panic!("the glow is a filled path"),
            };
            assert!(
                clip.width <= viewport.width + 1.0 && clip.height <= viewport.height + 1.0,
                "{edge:?} band {clip:?} escapes the viewport"
            );
            let touches_edge = match edge {
                GlowEdge::Top => (clip.y - viewport.y).abs() < 1.0,
                GlowEdge::Bottom => {
                    ((clip.y + clip.height) - (viewport.y + viewport.height)).abs() < 1.0
                }
                GlowEdge::Left => (clip.x - viewport.x).abs() < 1.0,
                GlowEdge::Right => {
                    ((clip.x + clip.width) - (viewport.x + viewport.width)).abs() < 1.0
                }
            };
            assert!(touches_edge, "{edge:?} band {clip:?} is not on its edge");
            let _ = bounds;
        }
    }

    #[test]
    fn an_idle_glow_paints_nothing() {
        let mut scene = Scene::new();
        OverscrollGlow::default().paint(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            GlowEdge::Top,
            Color::rgb(1.0, 1.0, 1.0),
            &mut scene,
        );
        assert!(scene.is_empty());
    }

    #[test]
    fn the_refused_direction_picks_the_edge() {
        assert_eq!(edge_for(true, -5.0), GlowEdge::Top);
        assert_eq!(edge_for(true, 5.0), GlowEdge::Bottom);
        assert_eq!(edge_for(false, -5.0), GlowEdge::Left);
        assert_eq!(edge_for(false, 5.0), GlowEdge::Right);
    }

    #[test]
    fn the_cross_axis_is_measured_along_the_edge() {
        let viewport = Rect::new(10.0, 20.0, 400.0, 600.0);
        let (offset, extent) = cross_axis(viewport, GlowEdge::Top, Point::new(210.0, 300.0));
        assert_eq!((offset, extent), (200.0, 400.0));
        let (offset, extent) = cross_axis(viewport, GlowEdge::Left, Point::new(210.0, 320.0));
        assert_eq!((offset, extent), (300.0, 600.0));
        // A pointer outside the viewport still lands on the edge.
        let (offset, _) = cross_axis(viewport, GlowEdge::Top, Point::new(-500.0, 0.0));
        assert_eq!(offset, 0.0);
    }

    #[test]
    fn all_four_edges_are_addressable_and_start_quiet() {
        let mut glows = ScrollGlows::default();
        assert!(glows.is_idle());
        glows.edge_mut(GlowEdge::Bottom).absorb_impact(2000.0);
        assert!(!glows.is_idle());
        assert!(glows.edge(GlowEdge::Top).is_idle(), "only one edge reacts");
        while glows.advance(1.0 / 60.0) {}
        assert!(glows.is_idle());
    }
}
