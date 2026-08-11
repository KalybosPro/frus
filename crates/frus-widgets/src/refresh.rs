//! [`Refresh`] — pull a list past its top edge to ask for fresh data.
//!
//! ```ignore
//! Refresh::new(list)
//!     .on_refresh(Msg::Reload)
//!     .refreshing(self.loading)
//! ```
//!
//! ## What drives it
//!
//! Nothing new is measured. Milestone 279 established that
//! `ScrollPhysics::apply_boundary_conditions` returns the movement the physics
//! **refused** — the distance the finger asked for and did not get. The overscroll glow
//! acknowledges that distance; a refresh area *accumulates* it, and past a threshold
//! turns it into a message.
//!
//! Where a `Refresh` is listening, the glow on that edge stands down. Two answers to
//! one gesture would say the same thing twice, and the indicator already is the
//! acknowledgement.
//!
//! ## Who decides when it is over
//!
//! The framework reports that the user asked; the application decides when the answer
//! has arrived. `on_refresh` is dispatched on release, and the indicator keeps spinning
//! for exactly as long as the tree is rebuilt with `refreshing(true)`. There is no
//! future to await and no callback to complete: the flag in the application's own state
//! is the single source of truth, which is also what makes the whole thing testable
//! without a clock.
//!
//! The message fires **on release** rather than when the indicator finishes snapping
//! into place. The snap takes 150 ms, and there is no reason to make a network request
//! wait for an animation.

use frus_core::{Color, Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Status, WidgetId};
use crate::spinner::{paint_activity_ring, RingMode};
use crate::theme::Theme;
use crate::widget::Widget;

/// How far the finger must drag to fill the indicator, as a fraction of the
/// scrollable's own extent. A long list and a short one therefore ask for the same
/// *proportional* gesture rather than the same number of pixels.
pub const DRAG_EXTENT_FRACTION: f32 = 0.25;

/// How far past the resting displacement the drag may push the indicator.
pub const DRAG_SIZE_FACTOR_LIMIT: f32 = 1.5;

/// The fill at which the pull is **armed** — releasing now triggers a refresh — and
/// equally the fill the indicator settles back to while it spins.
pub const ARM_AT: f32 = 1.0 / DRAG_SIZE_FACTOR_LIMIT;

/// Seconds for a released, armed pull to settle to its resting place.
pub const SNAP_TIME: f32 = 0.150;

/// Seconds for the indicator to scale away when the refresh is over, and equally for
/// an unarmed pull to slide back out of sight.
pub const SCALE_TIME: f32 = 0.200;

/// How much of the ring a full drag fills: three quarters, so that a completed pull is
/// visibly *not* a completed circle — the circle is what spinning means.
pub const DRAG_FILL: f32 = 0.75;

/// Where a refresh area is in its cycle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RefreshPhase {
    /// A finger is pulling, but not far enough yet.
    Drag,
    /// A finger is pulling past the threshold: releasing now refreshes.
    Armed,
    /// Released while armed: settling to the resting displacement.
    Snap,
    /// The application is working, and says so.
    Refresh,
    /// The work is over: scaling away.
    Done,
    /// Released short of the threshold: sliding back out of sight.
    Cancel,
}

impl RefreshPhase {
    /// `true` while a finger still owns the pull.
    pub fn is_dragging(&self) -> bool {
        matches!(self, RefreshPhase::Drag | RefreshPhase::Armed)
    }
}

/// The retained state of one refresh area, held by the [`crate::Runtime`].
#[derive(Copy, Clone, Debug)]
pub struct RefreshPull {
    phase: RefreshPhase,
    /// Accumulated overscroll, in px — the raw thing the finger did.
    drag: f32,
    /// How full the indicator is, `0..=1`. `ARM_AT` is its resting place.
    position: f32,
    /// How far the indicator has scaled away, `0..=1`; `1` is gone.
    scaled_out: f32,
    /// Where an animated phase started from, and how far through it is (seconds).
    from: f32,
    elapsed: f32,
    /// Turns of the spinning ring — kept here rather than read from the global clock,
    /// so the ring starts from rest each time rather than from wherever the clock was.
    turns: f32,
}

impl Default for RefreshPull {
    fn default() -> Self {
        Self {
            phase: RefreshPhase::Drag,
            drag: 0.0,
            position: 0.0,
            scaled_out: 0.0,
            from: 0.0,
            elapsed: 0.0,
            turns: 0.0,
        }
    }
}

impl RefreshPull {
    /// Where the area is in its cycle.
    pub fn phase(&self) -> RefreshPhase {
        self.phase
    }

    /// How full the indicator is, `0..=1`.
    pub fn position(&self) -> f32 {
        self.position
    }

    /// How much of the indicator is still there, `0..=1` — `Done` shrinks it to zero.
    pub fn scale(&self) -> f32 {
        1.0 - self.scaled_out
    }

    /// `true` once the area has nothing left to show, so the runtime can forget it.
    pub fn is_idle(&self) -> bool {
        (self.phase == RefreshPhase::Cancel && self.position <= 0.0)
            || (self.phase == RefreshPhase::Done && self.scaled_out >= 1.0)
    }

    /// Accumulates `overscroll` px of refused movement over a scrollable of
    /// `extent` px, and arms the pull once it is full enough.
    ///
    /// A pull that has already been released is not resumed: the finger that started
    /// it is gone, and a second finger arriving mid-animation should start its own.
    fn pull(&mut self, overscroll: f32, extent: f32) {
        if !self.phase.is_dragging() || extent <= 0.0 {
            return;
        }
        self.drag = (self.drag + overscroll).max(0.0);
        let full = extent * DRAG_EXTENT_FRACTION;
        let mut value = self.drag / full;
        // An armed pull cannot fall back below the threshold by easing off; only
        // letting go ends it. Otherwise the indicator would flicker in and out of
        // "release me now" as the finger wavers around the line.
        if self.phase == RefreshPhase::Armed {
            value = value.max(ARM_AT);
        }
        self.position = value.clamp(0.0, 1.0);
        if self.phase == RefreshPhase::Drag && self.position >= ARM_AT {
            self.phase = RefreshPhase::Armed;
        }
    }

    /// The pull is called off without a release — the list scrolled away from its top
    /// edge, or the gesture was cancelled outright. It slides back as an unarmed
    /// release would, and asks for nothing.
    fn cancel(&mut self) {
        if self.phase.is_dragging() {
            self.begin(RefreshPhase::Cancel);
        }
    }

    /// The finger lets go. Returns `true` when the pull was armed, and so when the
    /// application should be told to refresh.
    fn release(&mut self) -> bool {
        if !self.phase.is_dragging() {
            return false;
        }
        let armed = self.phase == RefreshPhase::Armed;
        self.phase = if armed {
            RefreshPhase::Snap
        } else {
            RefreshPhase::Cancel
        };
        self.from = self.position;
        self.elapsed = 0.0;
        armed
    }

    /// Advances by `dt` seconds, told whether the application still says it is
    /// working. Returns `true` while anything is still moving.
    fn advance(&mut self, dt: f32, refreshing: bool) -> bool {
        match self.phase {
            // A finger owns the pull; nothing moves on its own.
            RefreshPhase::Drag | RefreshPhase::Armed => false,
            RefreshPhase::Snap => {
                self.elapsed += dt;
                let t = (self.elapsed / SNAP_TIME).clamp(0.0, 1.0);
                self.position = self.from + (ARM_AT - self.from) * t;
                if t >= 1.0 {
                    // By now the application has had many frames to answer the
                    // message; its flag is authoritative. A refresh that needed no
                    // work simply plays snap → done, which reads as "already up to
                    // date" rather than as a spinner that never spun.
                    self.begin(if refreshing {
                        RefreshPhase::Refresh
                    } else {
                        RefreshPhase::Done
                    });
                }
                true
            }
            RefreshPhase::Refresh => {
                self.turns += dt * SPIN_SPEED;
                if !refreshing {
                    self.begin(RefreshPhase::Done);
                }
                true
            }
            RefreshPhase::Done => {
                self.elapsed += dt;
                self.scaled_out = (self.elapsed / SCALE_TIME).clamp(0.0, 1.0);
                self.scaled_out < 1.0
            }
            RefreshPhase::Cancel => {
                self.elapsed += dt;
                let t = (self.elapsed / SCALE_TIME).clamp(0.0, 1.0);
                self.position = self.from * (1.0 - t);
                t < 1.0
            }
        }
    }

    /// Enters `phase`, remembering where the animated quantities start from.
    fn begin(&mut self, phase: RefreshPhase) {
        self.phase = phase;
        self.from = self.position;
        self.elapsed = 0.0;
    }
}

/// Turns per second of the ring while the application is working.
const SPIN_SPEED: f32 = 1.1;

/// What a [`Refresh`] widget tells the frame about itself: everything the shell and the
/// paint need, without either of them holding on to the widget.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RefreshSpec {
    /// The application says it is working.
    pub refreshing: bool,
    /// Where the indicator rests, in px below the edge, while it spins.
    pub displacement: f32,
    /// The indicator's diameter, in px.
    pub size: f32,
    /// The ring's colour; `None` takes the theme's primary.
    pub color: Option<Color>,
    /// The disc behind the ring; `None` takes the theme's surface.
    pub background: Option<Color>,
}

impl Default for RefreshSpec {
    fn default() -> Self {
        Self {
            refreshing: false,
            displacement: 40.0,
            size: 36.0,
            color: None,
            background: None,
        }
    }
}

/// One refresh area of the frame: where it is, and what it was configured with.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Refreshable {
    /// The area's identity, which is also the key of its retained pull.
    pub id: WidgetId,
    /// The box the indicator is drawn in — the child's, on screen.
    pub viewport: Rect,
    /// The widget's configuration this frame.
    pub spec: RefreshSpec,
}

/// Wraps a scrollable so that pulling it past its top edge asks for fresh data.
///
/// ```ignore
/// Refresh::new(Scroll::new().child(rows))
///     .on_refresh(Msg::Reload)
///     .refreshing(self.loading)
///     .color(theme.scheme.tertiary)     // every part is overridable
/// ```
///
/// The child does not have to be a `Scroll` directly — any scrollable anywhere inside
/// feeds the pull, which is what lets a screen keep its own layout around the list.
pub struct Refresh<Msg> {
    spec: RefreshSpec,
    message: Option<Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Refresh<Msg> {
    /// Makes `child` refreshable. Nothing happens until
    /// [`on_refresh`](Self::on_refresh) gives it a message to send.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            spec: RefreshSpec::default(),
            message: None,
            children: vec![Box::new(child)],
        }
    }

    /// The message dispatched when an armed pull is released.
    pub fn on_refresh(mut self, message: Msg) -> Self {
        self.message = Some(message);
        self
    }

    /// Whether the application is currently working. The indicator spins for exactly
    /// as long as this is `true`.
    pub fn refreshing(mut self, refreshing: bool) -> Self {
        self.spec.refreshing = refreshing;
        self
    }

    /// Where the indicator rests while it spins, in px below the top edge.
    pub fn displacement(mut self, displacement: f32) -> Self {
        self.spec.displacement = displacement;
        self
    }

    /// The indicator's diameter, in px.
    pub fn size(mut self, size: f32) -> Self {
        self.spec.size = size;
        self
    }

    /// The ring's colour, overriding the theme's primary.
    pub fn color(mut self, color: Color) -> Self {
        self.spec.color = Some(color);
        self
    }

    /// The disc behind the ring, overriding the theme's surface.
    pub fn background(mut self, color: Color) -> Self {
        self.spec.background = Some(color);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Refresh<Msg> {
    fn style(&self) -> Style {
        // A pass-through: the child decides the box, and the indicator is drawn over
        // it rather than laid out beside it.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // The indicator is painted *after* the subtree, by the walk — otherwise the
        // list would be drawn on top of it.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn refresh(&self) -> Option<RefreshSpec> {
        Some(self.spec)
    }

    fn on_refresh(&self) -> Option<Msg> {
        self.message.clone()
    }
}

/// Draws the indicator of one refresh area over its viewport.
///
/// The geometry is a single factor: `position × DRAG_SIZE_FACTOR_LIMIT` runs from 0 to
/// 1.5, and carries the indicator from just above the edge (invisible) to `displacement`
/// px below it (at rest) and half as far again (a drag that overshoots).
pub fn paint_refresh(
    scene: &mut Scene,
    area: &Refreshable,
    pull: &RefreshPull,
    theme: &Theme,
    clip: Rect,
) {
    let scale = pull.scale();
    if scale <= 0.0 || pull.position() <= 0.0 {
        return;
    }
    let size = area.spec.size * scale;
    let radius = size * 0.5;
    let factor = pull.position() * DRAG_SIZE_FACTOR_LIMIT;

    let cx = area.viewport.x + area.viewport.width * 0.5;
    // At `factor == 0` the disc sits entirely above the edge, hidden by the clip; at
    // `1` its centre is exactly `displacement` below it.
    let cy = area.viewport.y - radius + (area.spec.displacement + radius) * factor;

    // The colour fades in over the same span that arms the pull, so "fully coloured"
    // and "release me now" are the same moment — the one signal the eye can read
    // without counting pixels.
    let opacity = (pull.position() / ARM_AT).clamp(0.0, 1.0);
    let ring = area.spec.color.unwrap_or(theme.primary);
    let disc = area.spec.background.unwrap_or(theme.surface);

    // The indicator belongs to its area: it must not spill over whatever is above.
    let outer = clip.intersect(area.viewport);
    if outer.width <= 0.0 || outer.height <= 0.0 {
        return;
    }
    let previous = scene.current_clip();
    scene.set_clip(outer);

    scene.draw_rect(
        Rect::new(cx - radius, cy - radius, size, size),
        disc.fade(opacity),
        radius,
        0.0,
        Color::TRANSPARENT,
    );

    let mode = match pull.phase() {
        // Spinning is what "working" looks like; a filled ring is what "pull further"
        // looks like. They must not be confusable, so a drag never completes the ring.
        RefreshPhase::Refresh | RefreshPhase::Done => RingMode::Spinning { head: pull.turns },
        _ => RingMode::Filling {
            progress: pull.position() * DRAG_FILL,
        },
    };
    paint_activity_ring(
        scene,
        cx,
        cy,
        radius * 0.62,
        (radius * 0.16).max(1.0),
        ring.fade(opacity),
        mode,
    );

    scene.set_clip(previous);
}

/// Advances every refresh area of the frame by `dt`, and drops those that have gone
/// quiet. Returns `true` while any is still moving.
///
/// Kept here rather than in `runtime.rs` so that the state machine and the thing that
/// steps it stay in one file.
pub(crate) fn advance_all(
    pulls: &mut std::collections::HashMap<WidgetId, RefreshPull>,
    areas: &[Refreshable],
    dt: f32,
) -> bool {
    if pulls.is_empty() {
        return false;
    }
    let mut animating = false;
    pulls.retain(|id, pull| {
        // An area that has left the tree keeps no claim on the frame.
        let Some(area) = areas.iter().find(|a| a.id == *id) else {
            return false;
        };
        animating |= pull.advance(dt, area.spec.refreshing);
        !pull.is_idle()
    });
    animating
}

/// Feeds overscroll into the pull of `id`, creating it on the first push.
///
/// `overscroll` is signed: a bouncing edge hands back what the rubber band gives up as
/// the finger returns, and the indicator follows it in.
pub(crate) fn pull_into(
    pulls: &mut std::collections::HashMap<WidgetId, RefreshPull>,
    id: WidgetId,
    overscroll: f32,
    extent: f32,
) {
    if overscroll == 0.0 {
        return;
    }
    // A negative push has nothing to create: there is no pull to give back to.
    if overscroll < 0.0 && !pulls.contains_key(&id) {
        return;
    }
    pulls.entry(id).or_default().pull(overscroll, extent);
}

/// Calls off the pull of `id` without asking for anything.
pub(crate) fn cancel_of(
    pulls: &mut std::collections::HashMap<WidgetId, RefreshPull>,
    id: WidgetId,
) {
    if let Some(pull) = pulls.get_mut(&id) {
        pull.cancel();
    }
}

/// Ends the pull of `id`, returning `true` when it was armed and the application
/// should be told.
pub(crate) fn release_of(
    pulls: &mut std::collections::HashMap<WidgetId, RefreshPull>,
    id: WidgetId,
) -> bool {
    pulls
        .get_mut(&id)
        .map(RefreshPull::release)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A 800 px viewport: a full drag is 800 × 0.25 = 200 px, and the pull arms at
    /// two thirds of that.
    const EXTENT: f32 = 800.0;
    const FULL: f32 = EXTENT * DRAG_EXTENT_FRACTION;

    fn area(refreshing: bool) -> Refreshable {
        Refreshable {
            id: WidgetId::ROOT,
            viewport: Rect::new(0.0, 0.0, 400.0, EXTENT),
            spec: RefreshSpec {
                refreshing,
                ..RefreshSpec::default()
            },
        }
    }

    /// Runs `seconds` of frames at 60 Hz.
    fn run(pulls: &mut HashMap<WidgetId, RefreshPull>, refreshing: bool, seconds: f32) {
        let areas = [area(refreshing)];
        let mut left = seconds;
        while left > 0.0 {
            let dt = left.min(1.0 / 60.0);
            advance_all(pulls, &areas, dt);
            left -= dt;
        }
    }

    fn pull_of(pulls: &HashMap<WidgetId, RefreshPull>) -> RefreshPull {
        *pulls.get(&WidgetId::ROOT).expect("a pull")
    }

    #[test]
    fn a_short_pull_does_not_arm() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL * 0.3, EXTENT);
        assert_eq!(pull_of(&pulls).phase(), RefreshPhase::Drag);
        assert!(!release_of(&mut pulls, WidgetId::ROOT));
        assert_eq!(pull_of(&pulls).phase(), RefreshPhase::Cancel);
    }

    #[test]
    fn a_long_enough_pull_arms_and_fires() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL * ARM_AT + 1.0, EXTENT);
        assert_eq!(pull_of(&pulls).phase(), RefreshPhase::Armed);
        assert!(
            release_of(&mut pulls, WidgetId::ROOT),
            "releasing an armed pull asks for a refresh"
        );
        assert_eq!(pull_of(&pulls).phase(), RefreshPhase::Snap);
    }

    #[test]
    fn the_threshold_scales_with_the_viewport() {
        // The same 60 px on a short viewport arms; on a tall one it does not.
        let mut short = HashMap::new();
        pull_into(&mut short, WidgetId::ROOT, 60.0, 300.0);
        assert_eq!(pull_of(&short).phase(), RefreshPhase::Armed);

        let mut tall = HashMap::new();
        pull_into(&mut tall, WidgetId::ROOT, 60.0, EXTENT);
        assert_eq!(pull_of(&tall).phase(), RefreshPhase::Drag);
    }

    #[test]
    fn an_armed_pull_does_not_disarm_when_the_finger_eases_off() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL, EXTENT);
        assert_eq!(pull_of(&pulls).phase(), RefreshPhase::Armed);
        // The finger comes most of the way back.
        pull_into(&mut pulls, WidgetId::ROOT, -FULL * 0.9, EXTENT);
        assert_eq!(
            pull_of(&pulls).phase(),
            RefreshPhase::Armed,
            "only letting go ends an armed pull"
        );
        assert!(pull_of(&pulls).position() >= ARM_AT);
    }

    #[test]
    fn the_indicator_cannot_be_dragged_past_the_limit() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL * 10.0, EXTENT);
        assert_eq!(pull_of(&pulls).position(), 1.0);
    }

    #[test]
    fn a_released_pull_is_not_resumed_by_more_overscroll() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL, EXTENT);
        release_of(&mut pulls, WidgetId::ROOT);
        let settling = pull_of(&pulls).position();
        pull_into(&mut pulls, WidgetId::ROOT, FULL, EXTENT);
        assert_eq!(
            pull_of(&pulls).position(),
            settling,
            "the finger that started it is gone"
        );
    }

    #[test]
    fn a_cancelled_pull_slides_away_and_is_forgotten() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL * 0.3, EXTENT);
        release_of(&mut pulls, WidgetId::ROOT);
        run(&mut pulls, false, SCALE_TIME + 0.05);
        assert!(pulls.is_empty(), "nothing left to draw, nothing retained");
    }

    #[test]
    fn a_refresh_spins_for_as_long_as_the_application_says() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL, EXTENT);
        release_of(&mut pulls, WidgetId::ROOT);

        run(&mut pulls, true, SNAP_TIME + 0.05);
        assert_eq!(pull_of(&pulls).phase(), RefreshPhase::Refresh);
        assert!(
            (pull_of(&pulls).position() - ARM_AT).abs() < 1e-3,
            "it rests exactly where the snap put it"
        );

        // Still working, a whole second later.
        run(&mut pulls, true, 1.0);
        assert_eq!(pull_of(&pulls).phase(), RefreshPhase::Refresh);

        // The application clears its flag: the indicator scales away and is dropped.
        run(&mut pulls, false, SCALE_TIME + 0.05);
        assert!(pulls.is_empty());
    }

    #[test]
    fn the_ring_turns_while_it_spins() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL, EXTENT);
        release_of(&mut pulls, WidgetId::ROOT);
        run(&mut pulls, true, SNAP_TIME + 0.05);
        let before = pull_of(&pulls).turns;
        run(&mut pulls, true, 0.5);
        assert!(pull_of(&pulls).turns > before);
    }

    #[test]
    fn a_refresh_that_needed_no_work_still_goes_away() {
        // The application never raises its flag — a cache hit, say.
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL, EXTENT);
        release_of(&mut pulls, WidgetId::ROOT);
        run(&mut pulls, false, SNAP_TIME + SCALE_TIME + 0.1);
        assert!(
            pulls.is_empty(),
            "snap then done, rather than a spinner that never spun"
        );
    }

    #[test]
    fn an_area_that_leaves_the_tree_takes_its_pull_with_it() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL, EXTENT);
        release_of(&mut pulls, WidgetId::ROOT);
        advance_all(&mut pulls, &[], 1.0 / 60.0);
        assert!(pulls.is_empty());
    }

    #[test]
    fn a_pull_that_is_still_held_moves_on_its_own_not_at_all() {
        let mut pulls = HashMap::new();
        pull_into(&mut pulls, WidgetId::ROOT, FULL * 0.5, EXTENT);
        let held = pull_of(&pulls).position();
        run(&mut pulls, false, 0.5);
        assert_eq!(
            pull_of(&pulls).position(),
            held,
            "the finger owns it, exactly as it owns the scroll offset"
        );
    }

    // --- Through a real frame -------------------------------------------------

    /// A refresh area wrapping a scrollable taller than its box, as the root of the
    /// tree — so the area's identity is `WidgetId::ROOT` and the fixtures above key on
    /// the same thing.
    fn tree(refreshing: bool) -> Refresh<i32> {
        Refresh::new(
            crate::Scroll::<i32>::new()
                .width(400.0)
                .height(EXTENT)
                .child(crate::Container::new().width(400.0).height(EXTENT * 4.0)),
        )
        .on_refresh(1)
        .refreshing(refreshing)
    }

    fn frame(runtime: &crate::Runtime, refreshing: bool) -> crate::Ui<i32> {
        crate::build_ui(
            &tree(refreshing),
            frus_core::Size::new(400.0, EXTENT),
            runtime,
            &crate::Theme::dark(),
        )
    }

    /// The ring is the only thing the indicator draws that is round; counting those
    /// primitives is how these tests see it without asserting on exact pixels.
    fn ring_dots(ui: &crate::Ui<i32>) -> usize {
        ui.scene()
            .primitives()
            .iter()
            .filter(|p| {
                matches!(p, frus_core::Primitive::Rect { rect, .. }
                    if (rect.width - rect.height).abs() < 0.01 && rect.width < 12.0)
            })
            .count()
    }

    #[test]
    fn a_scrollable_inside_a_refresh_area_names_its_host() {
        let runtime = crate::Runtime::default();
        let ui = frame(&runtime, false);
        let area = ui.scroll_regions().first().expect("a scrollable");
        assert_eq!(
            area.refresh,
            Some(WidgetId::ROOT),
            "so the shell knows where to send what the physics refuses"
        );
    }

    #[test]
    fn a_scrollable_outside_one_names_nothing() {
        let runtime = crate::Runtime::default();
        let bare = crate::Scroll::<i32>::new()
            .width(400.0)
            .height(EXTENT)
            .child(crate::Container::new().width(400.0).height(EXTENT * 4.0));
        let ui = crate::build_ui(
            &bare,
            frus_core::Size::new(400.0, EXTENT),
            &runtime,
            &crate::Theme::dark(),
        );
        assert_eq!(
            ui.scroll_regions().first().expect("a scrollable").refresh,
            None
        );
    }

    #[test]
    fn the_area_reports_itself_with_the_flag_it_was_built_with() {
        let runtime = crate::Runtime::default();
        let ui = frame(&runtime, true);
        let areas = ui.refresh_areas();
        assert_eq!(areas.len(), 1);
        assert_eq!(areas[0].id, WidgetId::ROOT);
        assert!(areas[0].spec.refreshing);
        assert_eq!(areas[0].viewport.height, EXTENT);
    }

    #[test]
    fn nothing_is_drawn_until_something_is_pulled() {
        let runtime = crate::Runtime::default();
        assert_eq!(ring_dots(&frame(&runtime, false)), 0);
    }

    #[test]
    fn a_pull_draws_the_indicator_over_its_area() {
        let mut runtime = crate::Runtime::default();
        runtime.refresh_pull(WidgetId::ROOT, FULL, EXTENT);
        let ui = frame(&runtime, false);
        assert!(ring_dots(&ui) > 0, "a filled ring is showing");
        assert!(
            ui.wants_animation(),
            "a live pull owes the next frame to itself"
        );
    }

    #[test]
    fn the_indicator_comes_down_as_the_pull_grows() {
        let centre_y = |drag: f32| {
            let mut runtime = crate::Runtime::default();
            runtime.refresh_pull(WidgetId::ROOT, drag, EXTENT);
            let ui = frame(&runtime, false);
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Rect { rect, .. }
                        if (rect.width - rect.height).abs() < 0.01 =>
                    {
                        Some(rect.y + rect.height * 0.5)
                    }
                    _ => None,
                })
                .fold(f32::MIN, f32::max)
        };
        assert!(
            centre_y(FULL) > centre_y(FULL * 0.4),
            "further pulled, further down"
        );
    }

    #[test]
    fn an_area_with_no_pull_asks_for_no_frames() {
        let runtime = crate::Runtime::default();
        assert!(!frame(&runtime, false).wants_animation());
    }

    #[test]
    fn the_widget_reports_what_it_was_configured_with() {
        let widget = Refresh::<i32>::new(crate::Container::new())
            .on_refresh(7)
            .refreshing(true)
            .displacement(64.0)
            .size(20.0);
        let spec = Widget::<i32>::refresh(&widget).expect("a refresh area");
        assert!(spec.refreshing);
        assert_eq!(spec.displacement, 64.0);
        assert_eq!(spec.size, 20.0);
        assert_eq!(Widget::<i32>::on_refresh(&widget), Some(7));
    }
}
