//! [`DraggableScrollableSheet`]: a panel along the bottom that **follows the finger**
//! between the heights it may rest at, and hands the gesture to the list inside it.
//!
//! ```ignore
//! Stack::new().layer(body).layer(
//!     DraggableScrollableSheet::new(places)
//!         .min(0.25)
//!         .snap_sizes([0.5])
//!         .on_dismiss(Msg::CloseSheet),
//! )
//! ```
//!
//! The sheet fills the box it is given and draws nothing of its own: its panel sits along
//! that box's bottom edge, as tall as the share of it the sheet is at, and the content
//! decides what a sheet looks like.
//!
//! # Who owns the finger
//!
//! A sheet whose content scrolls is two surfaces under one finger, and neither can be
//! chosen at the press. The shell splits **every movement** between them, which is what
//! lets the gesture change hands without the finger lifting:
//!
//! - moving **up**, the sheet grows until it is as tall as it goes, and only what is left
//!   over scrolls the list — unless the list is already scrolled, in which case all of it
//!   does;
//! - moving **down**, the list scrolls back to its top first, and only what is left over
//!   lowers the sheet.
//!
//! On release the same question is asked of the velocity: a sheet between two stops, or
//! thrown while its list is at the top, settles; a list thrown downwards while scrolled,
//! or upwards under a sheet already at full height, flings.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;

use frus_core::{ClampingScrollSimulation, Point, Rect, Scene, Simulation, Tolerance};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::{Status, WidgetId};
use crate::theme::Theme;
use crate::widget::Widget;

/// The slowest a sheet travels to the stop it is settling on, in px/s — the reference's
/// figure. A release that barely moved would otherwise creep the last few pixels.
pub const SHEET_SNAP_MIN_SPEED: f32 = 1600.0;

/// What a [`DraggableScrollableSheet`] tells the frame about itself. Heights are shares
/// of the box the sheet fills, from `0.0` to `1.0`.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetSpec {
    /// The lowest height it rests at while it stays open.
    pub min: f32,
    /// The highest.
    pub max: f32,
    /// Where it starts, before any finger has moved it.
    pub initial: f32,
    /// Whether a release **settles on a stop**, rather than coasting to wherever its
    /// momentum runs out.
    pub snap: bool,
    /// Every height it may settle on when it snaps, in increasing order: `min`, the
    /// sizes in between, `max` — and `0.0` below them when it can be dismissed.
    pub stops: Vec<f32>,
    /// Whether it may be lowered past `min` to nothing, which closes it.
    pub dismissible: bool,
}

impl SheetSpec {
    /// The lowest it can be moved to: nothing when it can be dismissed, `min` otherwise.
    pub fn floor(&self) -> f32 {
        if self.dismissible {
            0.0
        } else {
            self.min.min(self.max)
        }
    }

    /// `size`, kept between the floor and `max`.
    pub fn clamp(&self, size: f32) -> f32 {
        size.clamp(self.floor(), self.max)
    }
}

/// One sheet of the frame: where its panel is, what it was built with, and the scroll
/// areas walked inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetArea {
    /// The panel's identity, and the key of its retained state.
    pub id: WidgetId,
    /// The panel, on screen.
    pub panel: Rect,
    /// The height the sheet's shares are shares **of**, in px.
    pub available: f32,
    /// Its configuration this frame.
    pub spec: SheetSpec,
    /// The scroll areas registered inside the panel. Only these hand a gesture to the
    /// sheet: an area elsewhere on the screen owns its own finger.
    pub areas: Vec<WidgetId>,
}

impl SheetArea {
    /// Whether `point` is on the panel — never, while it has no height.
    pub fn contains(&self, point: Point) -> bool {
        let p = self.panel;
        p.height > 0.0
            && point.x >= p.x
            && point.x < p.x + p.width
            && point.y >= p.y
            && point.y < p.y + p.height
    }
}

/// How one movement of the finger is shared out, in px: `grow` is how far it went
/// **up** (negative for down), `list_offset` how far the list inside is scrolled, `size`
/// how tall the sheet is, and `floor` and `max` how low and high it may go.
///
/// Returns `(sheet, list)`: how far the sheet grows, and how far the list's offset
/// grows. They always add up to `grow`, so no movement of the finger is lost — what the
/// sheet cannot take past its floor goes to the list, whose own physics refuses it at
/// its edge and says so.
pub fn split_sheet_drag(
    grow: f32,
    list_offset: f32,
    size: f32,
    floor: f32,
    max: f32,
) -> (f32, f32) {
    if grow > 0.0 {
        // A list already scrolled keeps the finger: the sheet grows only from the top.
        if list_offset > 0.0 {
            return (0.0, grow);
        }
        let sheet = grow.min((max - size).max(0.0));
        (sheet, grow - sheet)
    } else if grow < 0.0 {
        let down = -grow;
        let list_back = down.min(list_offset.max(0.0));
        let rest = down - list_back;
        let sheet = rest.min((size - floor).max(0.0));
        (-sheet, -(list_back + rest - sheet))
    } else {
        (0.0, 0.0)
    }
}

/// Whether a release at `velocity` px/s — positive growing the sheet — is the **sheet's**
/// to settle, or the list's to fling. The reference's rule.
///
/// The list keeps it when the finger was still over a sheet already at rest on a stop,
/// when it was thrown down while scrolled, or thrown up under a sheet already at full
/// height. Everything else settles the sheet.
pub fn sheet_takes_release(
    velocity: f32,
    list_offset: f32,
    size: f32,
    spec: &SheetSpec,
    available: f32,
) -> bool {
    let tolerance = Tolerance::PIXELS;
    let still = velocity.abs() <= tolerance.velocity;
    let px = size * available;
    let on_stop = !spec.snap
        || spec
            .stops
            .iter()
            .any(|stop| (stop * available - px).abs() <= tolerance.distance);
    let at_max = px >= spec.max * available - tolerance.distance;
    !((still && on_stop) || (velocity < 0.0 && list_offset > 0.0) || (velocity > 0.0 && at_max))
}

/// The motion of a sheet to the stop it is settling on: a **constant speed** — the
/// release's, or [`SHEET_SNAP_MIN_SPEED`] if that is slower — that stops dead on the
/// stop. The reference's own, and deliberately not a spring: a sheet that overshot its
/// stop would uncover and cover again whatever is behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapSimulation {
    start: f32,
    target: f32,
    velocity: f32,
}

impl SnapSimulation {
    /// From `position` towards `target`, released at `velocity`. The direction is the
    /// target's, not the velocity's: a slow release can settle against its own motion.
    pub fn new(position: f32, velocity: f32, target: f32) -> Self {
        let velocity = if target < position {
            velocity.min(-SHEET_SNAP_MIN_SPEED)
        } else {
            velocity.max(SHEET_SNAP_MIN_SPEED)
        };
        Self {
            start: position,
            target,
            velocity,
        }
    }

    /// Where it is heading.
    pub fn target(&self) -> f32 {
        self.target
    }
}

impl Simulation for SnapSimulation {
    fn x(&self, time: f32) -> f32 {
        let position = self.start + self.velocity * time;
        if (self.velocity >= 0.0 && position > self.target)
            || (self.velocity < 0.0 && position < self.target)
        {
            self.target
        } else {
            position
        }
    }

    fn dx(&self, time: f32) -> f32 {
        if self.is_done(time) {
            0.0
        } else {
            self.velocity
        }
    }

    fn is_done(&self, time: f32) -> bool {
        self.x(time) == self.target
    }
}

/// What carries a released sheet.
#[derive(Clone, Copy, Debug)]
enum Motion {
    /// To a stop.
    Snap(SnapSimulation),
    /// Wherever the momentum runs out, as a list would.
    Coast(ClampingScrollSimulation),
}

/// A motion under way, in px, with what it was started against.
#[derive(Clone, Copy, Debug)]
struct Settle {
    motion: Motion,
    elapsed: f32,
    available: f32,
    floor: f32,
    max: f32,
}

/// The retained height of one sheet, and whatever is moving it.
#[derive(Clone, Copy, Debug)]
pub struct SheetState {
    size: f32,
    held: bool,
    settle: Option<Settle>,
    dismissed: bool,
    available: f32,
}

impl SheetState {
    /// A sheet nothing has moved yet: at its initial height.
    pub fn new(spec: &SheetSpec) -> Self {
        Self {
            size: spec.clamp(spec.initial),
            held: false,
            settle: None,
            dismissed: false,
            available: 0.0,
        }
    }

    /// Its height, as a share of the box.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// `true` while a release is still carrying it.
    pub fn is_settling(&self) -> bool {
        self.settle.is_some()
    }

    /// The height its shares were last taken of, in px — kept so that a panel lowered
    /// to nothing under the finger can still be measured, and brought back.
    pub fn available(&self) -> f32 {
        self.available
    }

    /// A finger is on it: whatever was carrying it stops where it is.
    fn hold(&mut self) {
        self.held = true;
        self.settle = None;
    }

    /// Moves it by `delta`, a share of the box, within its floor and `max`.
    fn drag(&mut self, spec: &SheetSpec, delta: f32, available: f32) {
        self.hold();
        if available > 0.0 {
            self.available = available;
        }
        self.size = spec.clamp(self.size + delta);
        if self.size > 0.0 {
            self.dismissed = false;
        }
    }

    /// The finger lets go at `velocity` px/s, positive growing it, over a box
    /// `available` px tall.
    fn release(&mut self, spec: &SheetSpec, available: f32, velocity: f32) {
        self.held = false;
        self.settle = None;
        if available <= 0.0 {
            return;
        }
        self.available = available;
        let tolerance = Tolerance::PIXELS;
        let position = self.size * available;
        let motion = if spec.snap {
            let stops: Vec<f32> = spec.stops.iter().map(|stop| stop * available).collect();
            let target =
                crate::physics::snap_target(position, velocity, &stops, tolerance.velocity);
            Motion::Snap(SnapSimulation::new(position, velocity, target))
        } else if velocity.abs() > tolerance.velocity {
            Motion::Coast(ClampingScrollSimulation::new(position, velocity, tolerance))
        } else {
            return;
        };
        self.settle = Some(Settle {
            motion,
            elapsed: 0.0,
            available,
            floor: spec.floor() * available,
            max: spec.max * available,
        });
    }

    /// Advances by `dt` seconds. Returns `(still moving, dismissed on this frame)`.
    fn advance(&mut self, spec: &SheetSpec, dt: f32) -> (bool, bool) {
        if self.held {
            return (false, false);
        }
        let mut moving = false;
        if let Some(settle) = &mut self.settle {
            settle.elapsed += dt;
            let (x, done) = match settle.motion {
                Motion::Snap(snap) => (snap.x(settle.elapsed), snap.is_done(settle.elapsed)),
                Motion::Coast(coast) => {
                    let x = coast.x(settle.elapsed);
                    // A coast stops at the sheet's own ends, not the list's.
                    let out = x <= settle.floor || x >= settle.max;
                    (x, out || coast.is_done(settle.elapsed))
                }
            };
            self.size = x.clamp(settle.floor, settle.max) / settle.available;
            if done {
                self.settle = None;
            } else {
                moving = true;
            }
        }
        if self.settle.is_none() && spec.dismissible && !self.dismissed && self.size <= 1e-4 {
            self.size = 0.0;
            self.dismissed = true;
            return (moving, true);
        }
        (moving, false)
    }
}

/// The height of the sheet `id`, as a share of its box — its initial one until anything
/// has moved it.
pub(crate) fn size_of(
    states: &HashMap<WidgetId, SheetState>,
    id: WidgetId,
    spec: &SheetSpec,
) -> f32 {
    states
        .get(&id)
        .map(|state| state.size)
        .unwrap_or_else(|| spec.clamp(spec.initial))
}

/// Moves the sheet `id` by `delta`, a share of its box, creating its state on the first
/// move.
pub(crate) fn drag_into(
    states: &mut HashMap<WidgetId, SheetState>,
    id: WidgetId,
    spec: &SheetSpec,
    delta: f32,
    available: f32,
) {
    states
        .entry(id)
        .or_insert_with(|| SheetState::new(spec))
        .drag(spec, delta, available);
}

/// A finger lands on the sheet `id`: a settle in progress stops under it.
pub(crate) fn hold_of(states: &mut HashMap<WidgetId, SheetState>, id: WidgetId) {
    if let Some(state) = states.get_mut(&id) {
        state.hold();
    }
}

/// The finger lets go of the sheet `id`.
pub(crate) fn release_of(
    states: &mut HashMap<WidgetId, SheetState>,
    id: WidgetId,
    spec: &SheetSpec,
    available: f32,
    velocity: f32,
) {
    if let Some(state) = states.get_mut(&id) {
        state.release(spec, available, velocity);
    }
}

/// Advances every sheet of the frame, dropping the state of any that has left it — so a
/// sheet shown again starts from its initial height. Returns `(still moving, the sheets
/// dismissed on this frame)`.
pub(crate) fn advance_all(
    states: &mut HashMap<WidgetId, SheetState>,
    areas: &[SheetArea],
    dt: f32,
) -> (bool, Vec<WidgetId>) {
    if states.is_empty() {
        return (false, Vec::new());
    }
    let mut moving = false;
    let mut dismissed = Vec::new();
    states.retain(|id, state| {
        let Some(area) = areas.iter().find(|area| area.id == *id) else {
            return false;
        };
        let (still, gone) = state.advance(&area.spec, dt);
        moving |= still;
        if gone {
            dismissed.push(*id);
        }
        true
    });
    (moving, dismissed)
}

/// A sheet along the bottom of the box it fills, dragged between the heights it may rest
/// at — the reference's `DraggableScrollableSheet`.
///
/// Heights are shares of that box. It starts at [`initial`](Self::initial) (half),
/// moves between [`min`](Self::min) (a quarter) and [`max`](Self::max) (all of it), and
/// coasts where it is thrown unless it is told to [`snap`](Self::snap) — which giving it
/// [`snap_sizes`](Self::snap_sizes) does. Given [`on_dismiss`](Self::on_dismiss), it can
/// be lowered past `min` to nothing, and the message goes once it has arrived there.
///
/// The retained height is keyed by identity, and forgotten on the first frame the sheet
/// is not in the tree: an application that removes a dismissed sheet and shows it again
/// gets it back at its initial height.
pub struct DraggableScrollableSheet<Msg> {
    spec: SheetSpec,
    sizes: Vec<f32>,
    on_dismiss: Option<Msg>,
    child: RefCell<Option<Box<dyn Widget<Msg>>>>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> DraggableScrollableSheet<Msg> {
    /// A sheet holding `child`, which is laid out as tall as the sheet is.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut sheet = Self {
            spec: SheetSpec {
                min: 0.25,
                max: 1.0,
                initial: 0.5,
                snap: false,
                stops: Vec::new(),
                dismissible: false,
            },
            sizes: Vec::new(),
            on_dismiss: None,
            child: RefCell::new(Some(Box::new(child))),
            built: OnceCell::new(),
        };
        sheet.restop();
        sheet
    }

    /// The height it starts at, before a finger has moved it. Half, unset.
    #[must_use]
    pub fn initial(mut self, size: f32) -> Self {
        self.spec.initial = size.clamp(0.0, 1.0);
        self
    }

    /// The lowest height it rests at while it stays open. A quarter, unset.
    #[must_use]
    pub fn min(mut self, size: f32) -> Self {
        self.spec.min = size.clamp(0.0, 1.0);
        self.restop();
        self
    }

    /// The highest height it goes to. All of the box, unset.
    #[must_use]
    pub fn max(mut self, size: f32) -> Self {
        self.spec.max = size.clamp(0.0, 1.0);
        self.restop();
        self
    }

    /// Whether a release settles on a stop — `min`, `max` and any
    /// [`snap_sizes`](Self::snap_sizes) — rather than coasting.
    #[must_use]
    pub fn snap(mut self, snap: bool) -> Self {
        self.spec.snap = snap;
        self
    }

    /// Heights between `min` and `max` it may also settle on, and snapping turned on.
    #[must_use]
    pub fn snap_sizes(mut self, sizes: impl IntoIterator<Item = f32>) -> Self {
        self.sizes = sizes.into_iter().collect();
        self.spec.snap = true;
        self.restop();
        self
    }

    /// The message sent once the sheet has been lowered to nothing — which this also
    /// allows, below `min`. Unset, `min` is as low as it goes.
    #[must_use]
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self.spec.dismissible = true;
        self.restop();
        self
    }

    fn restop(&mut self) {
        let (min, max) = (self.spec.min.min(self.spec.max), self.spec.max);
        let mut stops = Vec::with_capacity(self.sizes.len() + 3);
        if self.spec.dismissible {
            stops.push(0.0);
        }
        stops.push(min);
        stops.extend(self.sizes.iter().copied().filter(|s| *s > min && *s < max));
        stops.push(max);
        stops.sort_by(f32::total_cmp);
        stops.dedup();
        self.spec.stops = stops;
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for DraggableScrollableSheet<Msg> {
    /// The whole box, with the panel along its bottom edge.
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
            justify: Justify::End,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| {
            let child = self.child.borrow_mut().take();
            vec![Box::new(SheetPanel {
                spec: self.spec.clone(),
                on_dismiss: self.on_dismiss.clone(),
                children: child.into_iter().collect(),
            })]
        })
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "DraggableScrollableSheet"
    }
}

/// The part that moves: as wide as the sheet, as tall as the runtime says, with the
/// content stretched over it.
struct SheetPanel<Msg> {
    spec: SheetSpec,
    on_dismiss: Option<Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for SheetPanel<Msg> {
    /// The height here is the resting one; the layout replaces it with the retained one
    /// (see `effective_style`).
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(self.spec.clamp(self.spec.initial)),
            overlap: true,
            align: Align::Stretch,
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

    fn sheet(&self) -> Option<&SheetSpec> {
        Some(&self.spec)
    }

    fn on_sheet_dismissed(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn debug_name(&self) -> &'static str {
        "SheetPanel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime};
    use frus_core::{Color, Primitive, Size};

    fn spec(snap: bool, dismissible: bool) -> SheetSpec {
        let mut sheet = DraggableScrollableSheet::<()>::new(Container::new()).min(0.25);
        if snap {
            sheet = sheet.snap_sizes([0.5]);
        }
        if dismissible {
            sheet = sheet.on_dismiss(());
        }
        sheet.spec
    }

    /// **Up, the sheet first; then the list** — and a list already scrolled keeps the
    /// finger, since a sheet grows only from the top of what it holds.
    #[test]
    fn up_grows_the_sheet_to_its_max_then_scrolls_the_list() {
        assert_eq!(
            split_sheet_drag(30.0, 0.0, 400.0, 200.0, 800.0),
            (30.0, 0.0)
        );
        assert_eq!(
            split_sheet_drag(30.0, 0.0, 790.0, 200.0, 800.0),
            (10.0, 20.0),
            "the handover inside one movement"
        );
        assert_eq!(
            split_sheet_drag(30.0, 0.0, 800.0, 200.0, 800.0),
            (0.0, 30.0)
        );
        assert_eq!(
            split_sheet_drag(30.0, 5.0, 400.0, 200.0, 800.0),
            (0.0, 30.0),
            "a scrolled list keeps it"
        );
    }

    /// **Down, the list back to its top first; then the sheet** — and what the sheet
    /// cannot take below its floor still goes somewhere: to the list, which refuses it at
    /// its edge.
    #[test]
    fn down_scrolls_the_list_home_then_lowers_the_sheet() {
        assert_eq!(
            split_sheet_drag(-30.0, 100.0, 400.0, 200.0, 800.0),
            (0.0, -30.0)
        );
        assert_eq!(
            split_sheet_drag(-30.0, 10.0, 400.0, 200.0, 800.0),
            (-20.0, -10.0),
            "the handover inside one movement"
        );
        assert_eq!(
            split_sheet_drag(-30.0, 0.0, 400.0, 200.0, 800.0),
            (-30.0, 0.0)
        );
        assert_eq!(
            split_sheet_drag(-30.0, 0.0, 210.0, 200.0, 800.0),
            (-10.0, -20.0),
            "past the floor, the rest to the list"
        );
        assert_eq!(split_sheet_drag(0.0, 10.0, 400.0, 200.0, 800.0), (0.0, 0.0));
    }

    /// **Who settles a release**, case by case, as the reference decides it.
    #[test]
    fn a_release_goes_to_the_sheet_unless_the_list_was_thrown() {
        let snapping = spec(true, false);
        // At rest on a stop, a still finger moves nothing.
        assert!(!sheet_takes_release(0.0, 0.0, 0.5, &snapping, 800.0));
        // Between stops, even a still finger settles it.
        assert!(sheet_takes_release(0.0, 0.0, 0.4, &snapping, 800.0));
        // Thrown down while scrolled: the list's.
        assert!(!sheet_takes_release(-900.0, 40.0, 1.0, &snapping, 800.0));
        // Thrown down with the list at its top: the sheet's.
        assert!(sheet_takes_release(-900.0, 0.0, 1.0, &snapping, 800.0));
        // Thrown up under a full sheet: the list's; under a half one, the sheet's.
        assert!(!sheet_takes_release(900.0, 0.0, 1.0, &snapping, 800.0));
        assert!(sheet_takes_release(900.0, 0.0, 0.5, &snapping, 800.0));
        // A coasting sheet has no stops to be off, so a still finger never moves it.
        assert!(!sheet_takes_release(
            0.0,
            0.0,
            0.4,
            &spec(false, false),
            800.0
        ));
    }

    /// **The snap is a constant speed, never slower than the floor, and it stops dead on
    /// the stop** — heading for the stop even against a slow release's own motion.
    #[test]
    fn a_snap_travels_at_least_the_minimum_speed_and_stops_on_the_stop() {
        let up = SnapSimulation::new(300.0, 100.0, 400.0);
        assert_eq!(up.dx(0.0), SHEET_SNAP_MIN_SPEED);
        assert!((up.x(0.05) - 380.0).abs() < 1e-3);
        // 300 + 1600 × 0.09 = 444: the first frame past the stop is on it, not beyond.
        assert_eq!(
            up.x(0.09),
            400.0,
            "the frame it passes the stop, on the stop"
        );
        assert_eq!(up.x(1.0), 400.0, "never past it");
        assert!(up.is_done(1.0) && up.dx(1.0) == 0.0);
        let fast = SnapSimulation::new(300.0, 4000.0, 400.0);
        assert_eq!(fast.dx(0.0), 4000.0, "a faster release keeps its speed");
        let against = SnapSimulation::new(300.0, 50.0, 200.0);
        assert_eq!(against.dx(0.0), -SHEET_SNAP_MIN_SPEED, "towards the stop");
        assert_eq!(against.x(1.0), 200.0);
    }

    fn settle(state: &mut SheetState, spec: &SheetSpec) -> bool {
        let mut dismissed = false;
        for _ in 0..240 {
            let (_, gone) = state.advance(spec, 1.0 / 60.0);
            dismissed |= gone;
        }
        dismissed
    }

    /// **Released, a snapping sheet arrives on a stop**: the nearer one when let go
    /// slowly, the next one its way when flicked.
    #[test]
    fn a_released_sheet_arrives_on_a_stop() {
        let snapping = spec(true, false);
        let mut state = SheetState::new(&snapping);
        state.drag(&snapping, -0.2, 800.0); // 0.3: nearer the quarter than the half
        state.release(&snapping, 800.0, 0.0);
        assert!(state.is_settling());
        assert!(!settle(&mut state, &snapping));
        assert_eq!(state.size(), 0.25);

        state.drag(&snapping, 0.05, 800.0); // 0.3 again, flicked up
        state.release(&snapping, 800.0, 300.0);
        settle(&mut state, &snapping);
        assert_eq!(state.size(), 0.5);
        assert!(!state.is_settling());
    }

    /// **A flick down from the lowest stop dismisses**, once, and only when it can: a
    /// sheet without a message stops at `min`.
    #[test]
    fn a_flick_down_from_the_bottom_stop_dismisses_once() {
        let closing = spec(true, true);
        let mut state = SheetState::new(&closing);
        state.drag(&closing, -0.24, 800.0); // 0.26: a flick down goes to the quarter
        state.release(&closing, 800.0, -1200.0);
        assert!(
            !settle(&mut state, &closing),
            "the next stop its way, not nothing"
        );
        assert_eq!(state.size(), 0.25);
        state.drag(&closing, -0.01, 800.0); // 0.24: under the lowest stop that stays open
        state.release(&closing, 800.0, -1200.0);
        assert!(settle(&mut state, &closing), "dismissed");
        assert_eq!(state.size(), 0.0);
        assert!(!settle(&mut state, &closing), "and not twice");

        let staying = spec(true, false);
        let mut state = SheetState::new(&staying);
        state.drag(&staying, -0.24, 800.0);
        state.release(&staying, 800.0, -1200.0);
        assert!(!settle(&mut state, &staying));
        assert_eq!(state.size(), 0.25);
    }

    /// **A held sheet stays where the finger is**, even at nothing: it closes on the
    /// release, not under a finger that may yet bring it back.
    #[test]
    fn a_held_sheet_is_not_dismissed_under_the_finger() {
        let closing = spec(true, true);
        let mut state = SheetState::new(&closing);
        state.drag(&closing, -1.0, 800.0);
        assert_eq!(state.size(), 0.0);
        assert!(!settle(&mut state, &closing));
    }

    /// **A coasting sheet goes where it is thrown** and stops at its own ends; a still
    /// release leaves it where it is.
    #[test]
    fn a_coasting_sheet_goes_where_it_is_thrown() {
        let coasting = spec(false, false);
        let mut state = SheetState::new(&coasting);
        state.drag(&coasting, 0.0, 800.0);
        state.release(&coasting, 800.0, 0.0);
        assert!(!state.is_settling());
        assert_eq!(state.size(), 0.5);
        state.release(&coasting, 800.0, 600.0);
        settle(&mut state, &coasting);
        assert!(
            state.size() > 0.55 && state.size() < 1.0,
            "{}",
            state.size()
        );
        state.release(&coasting, 800.0, 8000.0);
        settle(&mut state, &coasting);
        assert_eq!(state.size(), 1.0, "and stops at its max");
    }

    /// **A sheet that leaves the frame forgets its height**, so shown again it starts
    /// from its initial one.
    #[test]
    fn a_sheet_that_leaves_the_frame_forgets_its_height() {
        let snapping = spec(true, false);
        let id = WidgetId::ROOT.child(0);
        let mut states = HashMap::new();
        drag_into(&mut states, id, &snapping, 0.3, 800.0);
        let area = SheetArea {
            id,
            panel: Rect::new(0.0, 0.0, 10.0, 10.0),
            available: 800.0,
            spec: snapping.clone(),
            areas: Vec::new(),
        };
        advance_all(&mut states, std::slice::from_ref(&area), 0.016);
        assert_eq!(size_of(&states, id, &snapping), 0.8);
        advance_all(&mut states, &[], 0.016);
        assert_eq!(size_of(&states, id, &snapping), 0.5);
    }

    /// **The panel is as tall as the retained height, along the bottom, with the content
    /// stretched over it** — read off the scene.
    #[test]
    fn the_panel_sits_along_the_bottom_at_the_retained_height() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let sheet =
            || DraggableScrollableSheet::<()>::new(Container::new().color(red)).snap_sizes([0.5]);
        let painted = |runtime: &Runtime| {
            let root = sheet();
            let ui = build_ui(&root, Size::new(400.0, 800.0), runtime, &Theme::default());
            ui.scene().primitives().iter().find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == red => Some(*rect),
                _ => None,
            })
        };
        let at_rest = painted(&Runtime::default()).expect("the content is painted");
        assert_eq!(
            (at_rest.y, at_rest.height, at_rest.width),
            (400.0, 400.0, 400.0)
        );

        let mut runtime = Runtime::default();
        let panel = WidgetId::ROOT.child(0);
        drag_into(&mut runtime.sheets, panel, &sheet().spec, 0.25, 800.0);
        let raised = painted(&runtime).expect("the content is painted");
        assert_eq!((raised.y, raised.height), (200.0, 600.0));
    }
}
