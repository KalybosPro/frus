//! [`Dismissible`] — swipe an item aside to remove it.
//!
//! ```ignore
//! Dismissible::new(row)
//!     .height(56.0)
//!     .on_dismiss(Msg::Delete(id))
//!     .background(Container::new().color(theme.error))
//! ```
//!
//! ## The gesture is shared, not stolen
//!
//! A dismissible list item covers the whole row, and a list scrolls. The two gestures
//! start identically — a finger goes down and moves — so the question is not *who is on
//! top* but *which way the finger went*. The first movement past the drag threshold
//! decides: mostly sideways is a dismissal, mostly up or down is a scroll, and the loser
//! never sees the gesture at all. That decision lives in the shell (`app.rs`), because
//! it is the only place that knows about both.
//!
//! Without it, wrapping a row in a `Dismissible` would silently break scrolling for the
//! whole list — the failure being that the list simply stops moving, with nothing to
//! suggest why.
//!
//! ## The three acts
//!
//! | | |
//! |---|---|
//! | **drag** | the item follows the finger, revealing a background behind it |
//! | **settle** | released: it flies out the way it was going, or slides back |
//! | **collapse** | once out, its height shrinks to nothing, and *then* the message goes |
//!
//! The collapse is what keeps a list from jumping: the neighbours close the gap over
//! 300 ms instead of teleporting into it. The message is dispatched at the end, so the
//! application removes the item only once the hole it leaves has already closed.
//!
//! ## Sizing
//!
//! A `Dismissible` overlays its backgrounds under its child, which makes it a layout
//! **leaf** like [`crate::Stack`]: give it a height (or a width, on a vertical swipe).
//! A row in a `ListView`, which already has a fixed item height, is the usual case.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::{Status, WidgetId};
use crate::theme::Theme;
use crate::widget::Widget;

/// Fraction of the item's extent a drag must cover to dismiss on release.
pub const DISMISS_THRESHOLD: f32 = 0.4;

/// Least speed, in px/s, at which a release counts as a fling rather than a drop.
pub const MIN_FLING_VELOCITY: f32 = 700.0;

/// By how much the swipe axis must beat the other one for a fling to count. A fast
/// diagonal is not a swipe; this is what keeps a hurried scroll from throwing rows out.
pub const MIN_FLING_VELOCITY_DELTA: f32 = 400.0;

/// Seconds for a released item to fly out or slide back.
pub const MOVEMENT_TIME: f32 = 0.200;

/// Seconds for a dismissed item's box to shrink to nothing.
pub const RESIZE_TIME: f32 = 0.300;

/// Which way an item may be swiped, and which way it went.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DismissDirection {
    /// Towards the reading start — leftwards in LTR.
    ToStart,
    /// Towards the reading end — rightwards in LTR.
    ToEnd,
    /// Upwards.
    Up,
    /// Downwards.
    Down,
}

impl DismissDirection {
    /// `true` when this direction moves along the horizontal axis.
    pub fn is_horizontal(&self) -> bool {
        matches!(self, DismissDirection::ToStart | DismissDirection::ToEnd)
    }

    /// The sign this direction gives the offset: negative towards the start or up.
    pub fn sign(&self) -> f32 {
        match self {
            DismissDirection::ToStart | DismissDirection::Up => -1.0,
            DismissDirection::ToEnd | DismissDirection::Down => 1.0,
        }
    }
}

/// Which directions an item accepts.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum DismissAxis {
    /// Both ways along the horizontal axis — the default.
    #[default]
    Horizontal,
    /// Both ways along the vertical axis.
    Vertical,
    /// Towards the reading start only.
    ToStart,
    /// Towards the reading end only.
    ToEnd,
    /// Upwards only.
    Up,
    /// Downwards only.
    Down,
}

impl DismissAxis {
    /// `true` when the accepted directions lie along the horizontal axis. This is what
    /// the shell compares the finger's movement against.
    pub fn is_horizontal(&self) -> bool {
        matches!(
            self,
            DismissAxis::Horizontal | DismissAxis::ToStart | DismissAxis::ToEnd
        )
    }

    /// The direction a movement of `sign` would be, or `None` when that way is not
    /// accepted — which is how a one-way item refuses the wrong swipe outright, rather
    /// than letting it move and then snapping back.
    pub fn direction(&self, sign: f32) -> Option<DismissDirection> {
        let negative = sign < 0.0;
        match (self, negative) {
            (DismissAxis::Horizontal, true) | (DismissAxis::ToStart, true) => {
                Some(DismissDirection::ToStart)
            }
            (DismissAxis::Horizontal, false) | (DismissAxis::ToEnd, false) => {
                Some(DismissDirection::ToEnd)
            }
            (DismissAxis::Vertical, true) | (DismissAxis::Up, true) => Some(DismissDirection::Up),
            (DismissAxis::Vertical, false) | (DismissAxis::Down, false) => {
                Some(DismissDirection::Down)
            }
            _ => None,
        }
    }
}

/// Where an item is in its cycle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DismissPhase {
    /// A finger is moving it.
    Drag,
    /// Released: sliding back to rest.
    Restore,
    /// Released past the point of no return: flying out.
    Fly,
    /// Out of sight: the box is shrinking, and the message follows.
    Collapse,
}

/// The retained state of one dismissible item.
#[derive(Copy, Clone, Debug)]
pub struct DismissState {
    phase: DismissPhase,
    /// How far along its axis the item has moved, as a fraction of its extent.
    /// Negative towards the start or up.
    progress: f32,
    /// How much of the item's box is left, `1` down to `0`.
    extent_factor: f32,
    /// Where an animated phase started, and how far through it is.
    from: f32,
    elapsed: f32,
    /// The way it went, once that is settled.
    direction: Option<DismissDirection>,
}

impl Default for DismissState {
    fn default() -> Self {
        Self {
            phase: DismissPhase::Drag,
            progress: 0.0,
            extent_factor: 1.0,
            from: 0.0,
            elapsed: 0.0,
            direction: None,
        }
    }
}

impl DismissState {
    /// Where the item is in its cycle.
    pub fn phase(&self) -> DismissPhase {
        self.phase
    }

    /// How far it has moved, as a signed fraction of its extent.
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// How much of its box is left, `1` down to `0`.
    pub fn extent_factor(&self) -> f32 {
        self.extent_factor
    }

    /// The way it went, once settled.
    pub fn direction(&self) -> Option<DismissDirection> {
        self.direction
    }

    /// `true` while a finger owns the item.
    pub fn is_dragging(&self) -> bool {
        self.phase == DismissPhase::Drag
    }

    /// Moves the item by `delta` px along its axis, over an item of `extent` px.
    /// A direction the item does not accept is refused outright rather than moved and
    /// snapped back.
    fn drag(&mut self, delta: f32, extent: f32, axis: DismissAxis) {
        if self.phase != DismissPhase::Drag || extent <= 0.0 {
            return;
        }
        let next = self.progress + delta / extent;
        let allowed = match axis.direction(next.signum()) {
            Some(_) => next,
            // Not an accepted direction: it may return to rest, and no further.
            None => 0.0,
        };
        self.progress = allowed.clamp(-1.0, 1.0);
        self.direction = axis.direction(self.progress.signum());
    }

    /// The finger lets go, at `velocity` px/s along the swipe axis and `cross` px/s
    /// across it. Returns the direction the item is being dismissed in, or `None` when
    /// it slides back.
    ///
    /// A fling counts only if it is fast enough **and** clearly along the axis: a
    /// hurried diagonal is not a swipe, and without that test a fast scroll would throw
    /// rows out of the list.
    fn release(
        &mut self,
        velocity: f32,
        cross: f32,
        axis: DismissAxis,
        threshold: f32,
    ) -> Option<DismissDirection> {
        if self.phase != DismissPhase::Drag {
            return None;
        }
        let flung = self.progress != 0.0
            && velocity.abs() - cross.abs() >= MIN_FLING_VELOCITY_DELTA
            && velocity.abs() >= MIN_FLING_VELOCITY;
        let outcome = if flung {
            axis.direction(velocity.signum())
        } else if self.progress.abs() >= threshold {
            axis.direction(self.progress.signum())
        } else {
            None
        };
        self.direction = outcome;
        self.from = self.progress;
        self.elapsed = 0.0;
        self.phase = match outcome {
            Some(_) => DismissPhase::Fly,
            None => DismissPhase::Restore,
        };
        outcome
    }

    /// Advances by `dt` seconds. Returns `Some(direction)` on the single frame the
    /// collapse finishes — the moment the application should be told.
    fn advance(&mut self, dt: f32) -> (bool, Option<DismissDirection>) {
        match self.phase {
            DismissPhase::Drag => (false, None),
            DismissPhase::Restore => {
                self.elapsed += dt;
                let t = (self.elapsed / MOVEMENT_TIME).clamp(0.0, 1.0);
                self.progress = self.from * (1.0 - t);
                (t < 1.0, None)
            }
            DismissPhase::Fly => {
                self.elapsed += dt;
                let t = (self.elapsed / MOVEMENT_TIME).clamp(0.0, 1.0);
                let target = self.direction.map(|d| d.sign()).unwrap_or(0.0);
                self.progress = self.from + (target - self.from) * t;
                if t >= 1.0 {
                    self.phase = DismissPhase::Collapse;
                    self.elapsed = 0.0;
                }
                (true, None)
            }
            DismissPhase::Collapse => {
                self.elapsed += dt;
                let t = (self.elapsed / RESIZE_TIME).clamp(0.0, 1.0);
                self.extent_factor = 1.0 - t;
                if t >= 1.0 {
                    // The gap has finished closing; only now is the item's absence
                    // something the application should act on.
                    return (false, self.direction);
                }
                (true, None)
            }
        }
    }

    /// `true` once there is nothing left to show or animate.
    fn is_spent(&self) -> bool {
        (self.phase == DismissPhase::Restore && self.progress == 0.0)
            || (self.phase == DismissPhase::Collapse && self.extent_factor <= 0.0)
    }
}

/// What a [`Dismissible`] tells the frame about itself.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DismissSpec {
    /// Which ways the item may be swiped.
    pub axis: DismissAxis,
    /// Fraction of the extent a drag must cover to dismiss on release.
    pub threshold: f32,
}

impl Default for DismissSpec {
    fn default() -> Self {
        Self {
            axis: DismissAxis::Horizontal,
            threshold: DISMISS_THRESHOLD,
        }
    }
}

/// One dismissible item of the frame: where it is, and how it was configured.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Dismissable {
    /// The item's identity, and the key of its retained state.
    pub id: WidgetId,
    /// Its box on screen.
    pub rect: Rect,
    /// Its configuration this frame.
    pub spec: DismissSpec,
}

impl Dismissable {
    /// The extent a drag is measured against: the width for a sideways swipe, the
    /// height for an up-and-down one.
    pub fn extent(&self) -> f32 {
        if self.spec.axis.is_horizontal() {
            self.rect.width
        } else {
            self.rect.height
        }
    }
}

/// Swipe an item aside to remove it.
///
/// ```ignore
/// Dismissible::new(row)
///     .height(56.0)
///     .on_dismiss(Msg::Delete(id))
///     .background(Container::new().color(theme.error))
/// ```
///
/// The background is revealed as the item slides off it. Give a `secondary_background`
/// to show something different in each direction — the usual "archive one way, delete
/// the other".
///
/// Wrap each item in a [`crate::Keyed`] when the list can reorder: the retained
/// dismissal state is keyed by identity, and positional identity would hand a
/// half-swiped state to whichever row moved into that slot.
pub struct Dismissible<Msg> {
    spec: DismissSpec,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    message: Option<Msg>,
    to_start: Option<Msg>,
    to_end: Option<Msg>,
    /// `[background?, secondary_background?, child]` — the child is always last, so it
    /// paints over them.
    children: Vec<Box<dyn Widget<Msg>>>,
    /// How many of `children` are backgrounds.
    backgrounds: usize,
}

impl<Msg: Clone> Dismissible<Msg> {
    /// Makes `child` swipeable. Nothing is removed until
    /// [`on_dismiss`](Self::on_dismiss) gives it a message to send.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            spec: DismissSpec::default(),
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            message: None,
            to_start: None,
            to_end: None,
            children: vec![Box::new(child)],
            backgrounds: 0,
        }
    }

    /// The message dispatched once the item has flown out **and** its gap has closed,
    /// whichever way it went.
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.message = Some(message);
        self
    }

    /// The message for a swipe towards the reading start, overriding
    /// [`on_dismiss`](Self::on_dismiss) for that direction.
    pub fn on_dismiss_to_start(mut self, message: Msg) -> Self {
        self.to_start = Some(message);
        self
    }

    /// The message for a swipe towards the reading end.
    pub fn on_dismiss_to_end(mut self, message: Msg) -> Self {
        self.to_end = Some(message);
        self
    }

    /// Which ways the item may be swiped; horizontal by default.
    pub fn axis(mut self, axis: DismissAxis) -> Self {
        self.spec.axis = axis;
        self
    }

    /// Fraction of the extent a drag must cover to dismiss on release.
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.spec.threshold = threshold;
        self
    }

    /// What shows behind the item as it slides away.
    pub fn background(mut self, background: impl Widget<Msg> + 'static) -> Self {
        self.children.insert(self.backgrounds, Box::new(background));
        self.backgrounds += 1;
        self
    }

    /// What shows behind the item when it goes the **other** way. Requires a
    /// [`background`](Self::background) first, which covers the first direction.
    pub fn secondary_background(mut self, background: impl Widget<Msg> + 'static) -> Self {
        self.children.insert(self.backgrounds, Box::new(background));
        self.backgrounds += 1;
        self
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// How many of the children are backgrounds — the child itself is the last one.
    pub fn background_count(&self) -> usize {
        self.backgrounds
    }
}

impl<Msg: Clone> Widget<Msg> for Dismissible<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    // The backgrounds sit under the child at the same box, which is exactly a stack —
    // and a stack is a layout leaf whose children are laid out separately.
    fn stack(&self) -> bool {
        true
    }

    fn dismissible(&self) -> Option<DismissSpec> {
        Some(self.spec)
    }

    fn on_dismissed(&self, direction: DismissDirection) -> Option<Msg> {
        let specific = match direction {
            DismissDirection::ToStart | DismissDirection::Up => self.to_start.as_ref(),
            DismissDirection::ToEnd | DismissDirection::Down => self.to_end.as_ref(),
        };
        specific.or(self.message.as_ref()).cloned()
    }
}

/// The colour of the strip a background shows through, for the tests and for a caller
/// painting its own.
pub fn revealed_strip(rect: Rect, progress: f32) -> Rect {
    let travelled = (rect.width * progress).abs().min(rect.width);
    if progress < 0.0 {
        // Slid towards the start: the strip is uncovered on the end side.
        Rect::new(
            rect.x + rect.width - travelled,
            rect.y,
            travelled,
            rect.height,
        )
    } else {
        Rect::new(rect.x, rect.y, travelled, rect.height)
    }
}

/// Advances every dismissible of the frame by `dt`, dropping those that have finished.
/// Returns `(still animating, the items whose gap has just closed)`.
pub(crate) fn advance_all(
    states: &mut std::collections::HashMap<WidgetId, DismissState>,
    items: &[Dismissable],
    dt: f32,
) -> (bool, Vec<(WidgetId, DismissDirection)>) {
    if states.is_empty() {
        return (false, Vec::new());
    }
    let mut animating = false;
    let mut finished = Vec::new();
    states.retain(|id, state| {
        // An item that has left the tree keeps no claim on the frame — except while it
        // is collapsing, which is the one phase whose whole point is that the widget is
        // on its way out.
        if !items.iter().any(|i| i.id == *id) && state.phase != DismissPhase::Collapse {
            return false;
        }
        let (moving, done) = state.advance(dt);
        animating |= moving;
        if let Some(direction) = done {
            finished.push((*id, direction));
        }
        !state.is_spent()
    });
    (animating, finished)
}

/// Moves the item `id` by `delta` px, creating its state on the first move.
pub(crate) fn drag_into(
    states: &mut std::collections::HashMap<WidgetId, DismissState>,
    id: WidgetId,
    delta: f32,
    extent: f32,
    axis: DismissAxis,
) {
    states.entry(id).or_default().drag(delta, extent, axis);
}

/// Releases the item `id`, returning the direction it is being dismissed in.
pub(crate) fn release_of(
    states: &mut std::collections::HashMap<WidgetId, DismissState>,
    id: WidgetId,
    velocity: f32,
    cross: f32,
    axis: DismissAxis,
    threshold: f32,
) -> Option<DismissDirection> {
    states
        .get_mut(&id)
        .and_then(|s| s.release(velocity, cross, axis, threshold))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const EXTENT: f32 = 400.0;

    fn item(id: WidgetId) -> Dismissable {
        Dismissable {
            id,
            rect: Rect::new(0.0, 0.0, EXTENT, 56.0),
            spec: DismissSpec::default(),
        }
    }

    fn run(states: &mut HashMap<WidgetId, DismissState>, seconds: f32) -> Vec<DismissDirection> {
        let items = [item(WidgetId::ROOT)];
        let mut out = Vec::new();
        let mut left = seconds;
        while left > 0.0 {
            let dt = left.min(1.0 / 60.0);
            let (_, done) = advance_all(states, &items, dt);
            out.extend(done.into_iter().map(|(_, d)| d));
            left -= dt;
        }
        out
    }

    fn state(states: &HashMap<WidgetId, DismissState>) -> DismissState {
        *states.get(&WidgetId::ROOT).expect("a state")
    }

    #[test]
    fn a_short_swipe_slides_back() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.2,
            EXTENT,
            DismissAxis::Horizontal,
        );
        assert!(release_of(
            &mut states,
            WidgetId::ROOT,
            0.0,
            0.0,
            DismissAxis::Horizontal,
            DISMISS_THRESHOLD
        )
        .is_none());
        assert_eq!(state(&states).phase(), DismissPhase::Restore);
        assert!(run(&mut states, MOVEMENT_TIME + 0.05).is_empty());
        assert!(
            states.is_empty(),
            "nothing retained once it is back at rest"
        );
    }

    #[test]
    fn a_long_enough_swipe_dismisses() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.5,
            EXTENT,
            DismissAxis::Horizontal,
        );
        let direction = release_of(
            &mut states,
            WidgetId::ROOT,
            0.0,
            0.0,
            DismissAxis::Horizontal,
            DISMISS_THRESHOLD,
        );
        assert_eq!(direction, Some(DismissDirection::ToEnd));
        assert_eq!(state(&states).phase(), DismissPhase::Fly);
    }

    #[test]
    fn the_message_waits_for_the_gap_to_close() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.5,
            EXTENT,
            DismissAxis::Horizontal,
        );
        release_of(
            &mut states,
            WidgetId::ROOT,
            0.0,
            0.0,
            DismissAxis::Horizontal,
            DISMISS_THRESHOLD,
        );

        // Flying out: nothing announced yet.
        assert!(run(&mut states, MOVEMENT_TIME + 0.01).is_empty());
        assert_eq!(state(&states).phase(), DismissPhase::Collapse);
        // The collapse has only just begun — the frame that ended the flight started it,
        // so a frame or two of it has already run.
        assert!(state(&states).extent_factor() > 0.9);

        // Half way through the collapse the box is half gone, still nothing announced.
        assert!(run(&mut states, RESIZE_TIME * 0.5).is_empty());
        let half = state(&states).extent_factor();
        assert!(
            (0.3..0.7).contains(&half),
            "the neighbours are closing the gap, got {half}"
        );

        let done = run(&mut states, RESIZE_TIME);
        assert_eq!(done, vec![DismissDirection::ToEnd]);
        assert!(states.is_empty());
    }

    #[test]
    fn a_fast_flick_dismisses_from_a_short_drag() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.05,
            EXTENT,
            DismissAxis::Horizontal,
        );
        let direction = release_of(
            &mut states,
            WidgetId::ROOT,
            1500.0,
            0.0,
            DismissAxis::Horizontal,
            DISMISS_THRESHOLD,
        );
        assert_eq!(direction, Some(DismissDirection::ToEnd));
    }

    #[test]
    fn a_fast_diagonal_is_not_a_swipe() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.05,
            EXTENT,
            DismissAxis::Horizontal,
        );
        // Fast enough sideways, but going down almost as fast: a hurried scroll.
        let direction = release_of(
            &mut states,
            WidgetId::ROOT,
            1500.0,
            1400.0,
            DismissAxis::Horizontal,
            DISMISS_THRESHOLD,
        );
        assert_eq!(direction, None, "this must not throw the row out");
    }

    #[test]
    fn a_flick_the_other_way_wins_over_the_drag_so_far() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.45,
            EXTENT,
            DismissAxis::Horizontal,
        );
        // Dragged past the threshold one way, then flicked back the other.
        let direction = release_of(
            &mut states,
            WidgetId::ROOT,
            -1500.0,
            0.0,
            DismissAxis::Horizontal,
            DISMISS_THRESHOLD,
        );
        assert_eq!(direction, Some(DismissDirection::ToStart));
    }

    #[test]
    fn a_one_way_item_refuses_the_wrong_direction_outright() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            -EXTENT * 0.5,
            EXTENT,
            DismissAxis::ToEnd,
        );
        assert_eq!(
            state(&states).progress(),
            0.0,
            "it does not move, rather than moving and snapping back"
        );
    }

    #[test]
    fn an_item_cannot_be_dragged_past_its_own_width() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 5.0,
            EXTENT,
            DismissAxis::Horizontal,
        );
        assert_eq!(state(&states).progress(), 1.0);
    }

    #[test]
    fn a_held_item_moves_on_its_own_not_at_all() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.3,
            EXTENT,
            DismissAxis::Horizontal,
        );
        let held = state(&states).progress();
        run(&mut states, 0.5);
        assert_eq!(state(&states).progress(), held);
    }

    #[test]
    fn a_collapsing_item_finishes_even_after_it_leaves_the_tree() {
        let mut states = HashMap::new();
        drag_into(
            &mut states,
            WidgetId::ROOT,
            EXTENT * 0.5,
            EXTENT,
            DismissAxis::Horizontal,
        );
        release_of(
            &mut states,
            WidgetId::ROOT,
            0.0,
            0.0,
            DismissAxis::Horizontal,
            DISMISS_THRESHOLD,
        );
        run(&mut states, MOVEMENT_TIME + 0.01);
        assert_eq!(state(&states).phase(), DismissPhase::Collapse);

        // The application removed the row early; the collapse still has to finish, or
        // the message that tells it to remove the row would never arrive.
        let mut done = Vec::new();
        for _ in 0..40 {
            let (_, d) = advance_all(&mut states, &[], 1.0 / 60.0);
            done.extend(d);
        }
        assert_eq!(done.len(), 1);
    }

    #[test]
    fn the_revealed_strip_follows_the_item_off() {
        let rect = Rect::new(10.0, 0.0, 100.0, 50.0);
        let to_end = revealed_strip(rect, 0.25);
        assert_eq!((to_end.x, to_end.width), (10.0, 25.0));
        let to_start = revealed_strip(rect, -0.25);
        assert_eq!((to_start.x, to_start.width), (85.0, 25.0));
    }

    #[test]
    fn the_widget_reports_the_message_for_the_direction_it_went() {
        let widget = Dismissible::<i32>::new(crate::Container::new())
            .on_dismiss(1)
            .on_dismiss_to_start(2);
        assert_eq!(
            Widget::<i32>::on_dismissed(&widget, DismissDirection::ToStart),
            Some(2)
        );
        assert_eq!(
            Widget::<i32>::on_dismissed(&widget, DismissDirection::ToEnd),
            Some(1),
            "the general message covers the direction with none of its own"
        );
    }

    #[test]
    fn the_child_is_the_last_of_the_children() {
        let widget = Dismissible::<i32>::new(crate::Text::new("row"))
            .background(crate::Container::new())
            .secondary_background(crate::Container::new());
        assert_eq!(widget.background_count(), 2);
        assert_eq!(Widget::<i32>::children(&widget).len(), 3);
        assert_eq!(
            Widget::<i32>::children(&widget)[2].debug_name(),
            "Text",
            "the child paints over its backgrounds"
        );
    }
}
