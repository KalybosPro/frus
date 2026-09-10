//! [`AnimatedSwitcher`]: moves between **two different children**, rather than between
//! two values of one.
//!
//! Every other implicit animation here moves a number — an opacity, a size, a pin — and
//! the widget carrying it stays the same widget throughout. A switcher is asked for
//! something else: the count was 3 and is now 4, and for a moment **both** are on the
//! screen, the 3 fading out while the 4 fades in. So something has to keep the 3.
//!
//! ## What is kept is the value, not the widget
//!
//! In the reference the switcher holds on to the old child *widget*, which works because
//! that widget outlives the frame it was made in. Here nothing does: the view is a pure
//! function of the application's state, the tree is rebuilt from it, and a boxed widget
//! can be neither kept nor cloned. What the application gave to *make* the child, though,
//! is plain data. So a switcher is handed a **value and a way to build a child from one**:
//!
//! ```ignore
//! AnimatedSwitcher::new(0.25, app.count, |n| text(n.to_string()))
//! ```
//!
//! and it is the values that the runtime keeps, under the switcher's identity. The child
//! on its way out is **built again from its value every frame** it is on the screen — it
//! is a real subtree, laid out, painted and animating in its own right, not a picture of
//! what it was. A value changing is what a switch *is*: equal values are the same child,
//! whatever else changed around them, which is the reference's key.
//!
//! ## Where it happens, and what it costs
//!
//! A new value can only be noticed while the tree is being **built** — that is the only
//! moment both it and the one before it are at hand — so the switcher does its work in
//! [`Widget::build_in`], which is handed its identity and the runtime. What it builds
//! carries the progress of each child as a plain number, through a **transition** of the
//! caller's choosing — a fade by default, any of the explicit transitions otherwise — so
//! while a switch is in flight the tree has to be built again each frame rather than only
//! painted. That is what a route transition already costs here, for the same reason; it
//! stops the frame the switch settles.
//!
//! The children on their way out take **no input** and are **not announced**: a tap on a
//! number that is fading away would be a tap on a state the application has already left,
//! and a screen reader reading it would be reading that state out.

use std::any::Any;
use std::cell::OnceCell;
use std::rc::Rc;

use frus_core::{Curve, Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Status, WidgetId};
use crate::runtime::Runtime;
use crate::theme::Theme;
use crate::widget::Widget;

/// Builds a child from a value the switcher has kept, or `None` if the value is not the
/// kind this switcher builds from (a different switcher came to stand in the same place).
type Build<Msg> = Box<dyn Fn(&dyn Any) -> Option<Box<dyn Widget<Msg>>>>;
/// Wraps a child in its progress: `1` in place, `0` gone.
type Transition<Msg> = Box<dyn Fn(Box<dyn Widget<Msg>>, f32) -> Box<dyn Widget<Msg>>>;
/// Arranges the children, the oldest first and the one arriving last.
type Arrange<Msg> = Box<dyn Fn(Vec<Box<dyn Widget<Msg>>>) -> Box<dyn Widget<Msg>>>;

/// **Moves between two different children** when the value it shows changes: the old one
/// leaves as the new one arrives — a fade by default.
///
/// ```
/// use frus_widgets::{AnimatedSwitcher, Text};
///
/// let count = 3;
/// let _shown: AnimatedSwitcher<()> =
///     AnimatedSwitcher::new(0.25, count, |n: &i32| Text::new(n.to_string()));
/// ```
///
/// It is handed the **value** a child is made from rather than the child, because a value
/// is what can be kept from one frame to the next — see the module documentation. Two
/// values that compare equal are the same child: nothing moves. A value that differs is a
/// switch, and the child that was showing leaves from wherever it had got to.
///
/// - [`AnimatedSwitcher::transition`] says **how** a child arrives and leaves. It is
///   handed the child and its progress, and any explicit transition fits:
///   `.transition(|child, t| ScaleTransition::new(t, child))`.
/// - [`AnimatedSwitcher::layout`] says **where** the children go while two are showing.
///   By default they overlap, centred, in a box as big as the largest of them.
/// - The first value is shown as it is, with no transition: an implicit animation adopts
///   what it is given on the frame it appears, here as everywhere.
///
/// An absent child is a value like any other: switch over an `Option` and build an empty
/// box for `None`, and a child fades out to nothing.
pub struct AnimatedSwitcher<Msg> {
    value: Rc<dyn Any>,
    same: fn(&dyn Any, &dyn Any) -> bool,
    build: Build<Msg>,
    timing: Timing,
    transition: Transition<Msg>,
    arrange: Arrange<Msg>,
    /// The arrangement, as the one-element slice [`Widget::children`] hands back. Composed
    /// once per tree, by the first walk that reaches this node.
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

/// Whether two kept values are the same child. Values of two different kinds never are.
fn same<T: PartialEq + 'static>(a: &dyn Any, b: &dyn Any) -> bool {
    match (a.downcast_ref::<T>(), b.downcast_ref::<T>()) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

impl<Msg: Clone + 'static> AnimatedSwitcher<Msg> {
    /// Shows the child `build` makes from `value`, and moves over `duration` seconds to
    /// the child of each new value.
    pub fn new<T, W>(duration: f32, value: T, build: impl Fn(&T) -> W + 'static) -> Self
    where
        T: PartialEq + 'static,
        W: Widget<Msg> + 'static,
    {
        Self {
            value: Rc::new(value),
            same: same::<T>,
            build: Box::new(move |kept: &dyn Any| {
                kept.downcast_ref::<T>()
                    .map(|value| Box::new(build(value)) as Box<dyn Widget<Msg>>)
            }),
            timing: Timing {
                duration,
                reverse: duration,
                switch_in: Curve::Linear,
                switch_out: Curve::Linear,
            },
            transition: Box::new(|child, progress| {
                Box::new(crate::transitions::FadeTransition::new(progress, child))
            }),
            arrange: Box::new(|layers| Box::new(Overlap { layers })),
            built: OnceCell::new(),
        }
    }

    /// How long a child takes to **leave**, when it is not as long as one takes to arrive.
    pub fn reverse_duration(mut self, seconds: f32) -> Self {
        self.timing.reverse = seconds;
        self
    }

    /// The curve a child **arrives** on. Linear by default, as in the reference.
    pub fn switch_in_curve(mut self, curve: Curve) -> Self {
        self.timing.switch_in = curve;
        self
    }

    /// The curve a child **leaves** on, read as its progress falls from where it was to
    /// nothing. Linear by default.
    pub fn switch_out_curve(mut self, curve: Curve) -> Self {
        self.timing.switch_out = curve;
        self
    }

    /// **How** a child arrives and leaves: handed the child and its progress — `1` in
    /// place, `0` gone — it returns the child as it should be drawn at that point. A fade
    /// by default.
    ///
    /// Both directions go through it, the leaving child with its progress falling, so a
    /// scale makes the old child shrink away as the new one grows.
    pub fn transition<W: Widget<Msg> + 'static>(
        mut self,
        transition: impl Fn(Box<dyn Widget<Msg>>, f32) -> W + 'static,
    ) -> Self {
        self.transition = Box::new(move |child, progress| Box::new(transition(child, progress)));
        self
    }

    /// **Where** the children go while more than one is showing: handed them oldest first,
    /// the arriving one last, it returns what holds them. By default they overlap, centred
    /// in a box as big as the largest.
    ///
    /// Each child arrives already keyed and wrapped in its transition, so the arrangement
    /// only has to place them — in the order given, if the arriving one is to be drawn on
    /// top.
    pub fn layout<W: Widget<Msg> + 'static>(
        mut self,
        layout: impl Fn(Vec<Box<dyn Widget<Msg>>>) -> W + 'static,
    ) -> Self {
        self.arrange = Box::new(move |layers| Box::new(layout(layers)));
        self
    }

    /// The children as they stand: from the runtime when a walk hands one over, and at rest
    /// otherwise — a bare traversal that reaches this node before any walk has is shown the
    /// value it was given, which is also what the first frame shows.
    fn compose(&self, at: Option<(WidgetId, &Runtime)>) -> Vec<Box<dyn Widget<Msg>>> {
        let shown = match at {
            Some((id, runtime)) => {
                runtime.switch_to(id, self.value.clone(), self.same, self.timing.clone())
            }
            None => vec![Shown {
                serial: 0,
                value: self.value.clone(),
                progress: 1.0,
                leaving: false,
            }],
        };
        let layers = shown
            .into_iter()
            .filter_map(|shown| {
                let child = (self.build)(shown.value.as_ref())?;
                let drawn = (self.transition)(child, shown.progress);
                // The same wrappers whether it is arriving or leaving, so that the subtree
                // keeps its identity — and whatever it retains — across the switch. A child
                // leaving is no longer the answer, to a finger or to a screen reader.
                let layer = crate::Keyed::new(
                    shown.serial,
                    crate::barrier::ExcludeSemantics::new(
                        crate::barrier::IgnorePointer::new(drawn).ignoring(shown.leaving),
                    )
                    .excluding(shown.leaving),
                );
                Some(Box::new(layer) as Box<dyn Widget<Msg>>)
            })
            .collect();
        vec![(self.arrange)(layers)]
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for AnimatedSwitcher<Msg> {
    fn build_in(&self, id: WidgetId, runtime: &Runtime, _theme: &Theme) {
        self.built.get_or_init(|| self.compose(Some((id, runtime))));
    }

    fn switches(&self) -> bool {
        true
    }

    fn style(&self) -> Style {
        // Nothing of its own: the arrangement's box is this node's box.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.compose(None))
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "AnimatedSwitcher"
    }
}

/// The default arrangement: every child in one cell, centred, the box as big as the
/// largest — the reference's centred stack. A [`crate::Stack`] cannot be it here: its
/// layers are laid out apart from it, so a stack with no size of its own is nothing.
struct Overlap<Msg> {
    layers: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for Overlap<Msg> {
    fn style(&self) -> Style {
        Style {
            overlap: true,
            align: frus_layout::Align::Center,
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.layers
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "AnimatedSwitcher"
    }
}

/// How long and on what curves a switcher's children move.
#[derive(Clone, Debug)]
pub(crate) struct Timing {
    duration: f32,
    reverse: f32,
    switch_in: Curve,
    switch_out: Curve,
}

/// One child as the tree is to be built with it.
pub(crate) struct Shown {
    serial: u64,
    value: Rc<dyn Any>,
    /// Through the curve: what the transition is handed.
    progress: f32,
    leaving: bool,
}

/// One child the runtime is keeping for a switcher.
struct Entry {
    /// Its identity among the switcher's children, for as long as it is on the screen.
    serial: u64,
    value: Rc<dyn Any>,
    /// Its own clock, linear: `0` gone, `1` in place. It runs up while the child arrives
    /// and down while it leaves, **from wherever it had got to** — a child cut off halfway
    /// in leaves from halfway, rather than jumping to full and fading from there.
    t: f32,
    leaving: bool,
}

/// What the runtime keeps for one switcher: the children on the screen, oldest first.
/// Only the last can be arriving; every one before it is leaving.
pub(crate) struct Switch {
    entries: Vec<Entry>,
    next: u64,
    timing: Timing,
    /// Moved since a tree was last built from it. What moved is baked into the children as
    /// numbers, so the frame has to be **built** again, not only painted.
    pub(crate) moved: bool,
}

impl Switch {
    /// A switcher seen for the first time: its value in place, with no transition.
    fn mounted(value: Rc<dyn Any>, timing: Timing) -> Self {
        Self {
            entries: vec![Entry {
                serial: 0,
                value,
                t: 1.0,
                leaving: false,
            }],
            next: 1,
            timing,
            moved: false,
        }
    }

    /// Takes in the value a tree was built with, and says what to show.
    pub(crate) fn meet(
        &mut self,
        value: Rc<dyn Any>,
        same: fn(&dyn Any, &dyn Any) -> bool,
        timing: Timing,
        still: bool,
    ) -> Vec<Shown> {
        self.timing = timing;
        let showing = self
            .entries
            .last()
            .is_some_and(|e| !e.leaving && same(e.value.as_ref(), value.as_ref()));
        if !showing {
            self.arrive(value, still);
        }
        self.moved = false;
        self.entries
            .iter()
            .map(|e| Shown {
                serial: e.serial,
                value: e.value.clone(),
                progress: if e.leaving {
                    self.timing.switch_out.transform(e.t)
                } else {
                    self.timing.switch_in.transform(e.t)
                },
                leaving: e.leaving,
            })
            .collect()
    }

    /// A new child: everything showing starts to leave, and it starts to arrive.
    fn arrive(&mut self, value: Rc<dyn Any>, still: bool) {
        for entry in &mut self.entries {
            entry.leaving = true;
        }
        // With the motion turned down, or no time to do it in, a child is simply gone —
        // and the new one simply there.
        if still || self.timing.reverse <= 0.0 {
            self.entries.clear();
        }
        let t = if still || self.timing.duration <= 0.0 {
            1.0
        } else {
            0.0
        };
        self.entries.push(Entry {
            serial: self.next,
            value,
            t,
            leaving: false,
        });
        self.next += 1;
    }

    /// Steps every child's clock by `dt`, and lets go of the ones that have left. Returns
    /// whether anything moved.
    pub(crate) fn advance(&mut self, dt: f32, still: bool) -> bool {
        let step = |duration: f32| {
            if still || duration <= 0.0 {
                f32::INFINITY
            } else {
                dt / duration
            }
        };
        let (arrive, leave) = (step(self.timing.duration), step(self.timing.reverse));
        let mut moved = false;
        for entry in &mut self.entries {
            let before = entry.t;
            entry.t = if entry.leaving {
                (entry.t - leave).max(0.0)
            } else {
                (entry.t + arrive).min(1.0)
            };
            // `dt` of nought moves nothing, however short the duration: it is the first
            // frame, not a step.
            if dt > 0.0 && entry.t != before {
                moved = true;
            } else {
                entry.t = before;
            }
        }
        let count = self.entries.len();
        self.entries.retain(|e| !e.leaving || e.t > 0.0);
        moved |= self.entries.len() != count;
        self.moved |= moved;
        moved
    }
}

impl Runtime {
    /// Hands a switcher what to show, noticing a new value — see [`Switch::meet`].
    pub(crate) fn switch_to(
        &self,
        id: WidgetId,
        value: Rc<dyn Any>,
        same: fn(&dyn Any, &dyn Any) -> bool,
        timing: Timing,
    ) -> Vec<Shown> {
        let still = self.still;
        let mut switchers = self.switchers.borrow_mut();
        switchers
            .entry(id)
            .or_insert_with(|| Switch::mounted(value.clone(), timing.clone()))
            .meet(value, same, timing, still)
    }

    /// Steps every [`AnimatedSwitcher`]'s children in and out, and forgets the switchers no
    /// longer in the tree. Returns whether a child is still moving.
    pub fn advance_switchers<Msg>(&mut self, root: &dyn Widget<Msg>, dt: f32) -> bool {
        fn find<Msg>(
            widget: &dyn Widget<Msg>,
            id: WidgetId,
            kept: &std::collections::HashMap<WidgetId, Switch>,
            present: &mut std::collections::HashSet<WidgetId>,
        ) {
            // Being at the identity is not enough: an identity is a position, and any
            // widget that comes to stand where the switcher stood has the same one.
            if widget.switches() && kept.contains_key(&id) {
                present.insert(id);
            }
            for (index, child) in widget.children().iter().enumerate() {
                find(
                    child.as_ref(),
                    crate::ui::child_id(id, index, child.as_ref()),
                    kept,
                    present,
                );
            }
        }
        let still = self.still;
        let switchers = self.switchers.get_mut();
        if switchers.is_empty() {
            return false;
        }
        // A switcher that has gone is forgotten, so that one appearing in its place later
        // is a mount — shown as it is — rather than a switch from what used to be there.
        let mut present = std::collections::HashSet::new();
        find(root, WidgetId::ROOT, switchers, &mut present);
        switchers.retain(|id, _| present.contains(id));
        let mut moving = false;
        for switch in switchers.values_mut() {
            moving |= switch.advance(dt, still);
        }
        moving
    }

    /// Whether a switcher has moved since the tree was last built, so that the next frame
    /// has to **build** the tree again rather than only paint it: what moved is carried by
    /// the children as numbers.
    pub fn switching(&self) -> bool {
        self.switchers.borrow().values().any(|s| s.moved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{build_ui, Ui};
    use crate::{Container, Flex, Keyed, Text};
    use frus_core::{Color, Point, Primitive, Size};

    const SIZE: Size = Size::new(200.0, 100.0);

    fn red() -> Color {
        Color::rgb(1.0, 0.0, 0.0)
    }
    fn blue() -> Color {
        Color::rgb(0.0, 0.0, 1.0)
    }
    fn green() -> Color {
        Color::rgb(0.0, 1.0, 0.0)
    }

    /// Value 1 is a wide red tile, 2 a narrow blue one, 3 a green one — each clickable
    /// with its own number and labelled with its own word.
    fn tile(value: u8) -> Container<i32> {
        let (color, width, word) = match value {
            1 => (red(), 80.0, "one"),
            2 => (blue(), 40.0, "two"),
            _ => (green(), 40.0, "three"),
        };
        Container::new()
            .width(width)
            .height(20.0)
            .color(color)
            .on_click(value as i32)
            .child(Text::new(word))
    }

    fn switcher(value: u8) -> AnimatedSwitcher<i32> {
        AnimatedSwitcher::new(0.2, value, |v: &u8| tile(*v))
    }

    /// In a row, so the switcher's box is what its children make it.
    fn view(value: u8) -> Flex<i32> {
        Flex::row().child(switcher(value))
    }

    /// One turn of the shell's loop: the tree built with `value`, then the clocks stepped.
    fn step(rt: &mut Runtime, value: u8, dt: f32) {
        let tree = view(value);
        let _ = build_ui(&tree, SIZE, rt, &Theme::dark());
        rt.advance_switchers(&tree, dt);
    }

    /// What the next frame draws.
    fn look(rt: &Runtime, value: u8) -> Ui<i32> {
        build_ui(&view(value), SIZE, rt, &Theme::dark())
    }

    /// Every filled rectangle, with the opacity it is drawn at through the layers above it.
    fn rects(ui: &Ui<i32>) -> Vec<(Rect, Color, f32)> {
        fn walk(primitives: &[Primitive], opacity: f32, out: &mut Vec<(Rect, Color, f32)>) {
            for primitive in primitives {
                match primitive {
                    Primitive::Rect { rect, color, .. } => out.push((*rect, *color, opacity)),
                    Primitive::Layer {
                        primitives,
                        opacity: layer,
                        ..
                    } => walk(primitives, opacity * layer, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(ui.scene().primitives(), 1.0, &mut out);
        out
    }

    /// How visible the tile of `color` is: `0` when it is not drawn at all.
    fn shown(ui: &Ui<i32>, color: Color) -> f32 {
        rects(ui)
            .iter()
            .filter(|(_, c, _)| *c == color)
            .map(|(_, _, opacity)| *opacity * color_alpha(color))
            .fold(0.0, f32::max)
    }

    fn color_alpha(color: Color) -> f32 {
        color.a
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    /// **The first value is shown as it is**: no fade in from nothing on the frame the
    /// switcher appears, the rule every implicit animation here keeps.
    #[test]
    fn the_first_value_is_shown_as_it_is() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);
        let ui = look(&rt, 1);
        assert_eq!(shown(&ui, red()), 1.0);
        assert_eq!(shown(&ui, blue()), 0.0);
    }

    /// **A new value crosses over**: at the change both are there, the old whole and the
    /// new not yet; halfway each is half; at the end only the new one is left in the tree.
    #[test]
    fn a_new_value_crosses_over_and_the_old_one_is_let_go() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);

        let ui = look(&rt, 2);
        assert_eq!(shown(&ui, red()), 1.0, "the old child, still whole");
        assert_eq!(shown(&ui, blue()), 0.0, "the new one, not yet");

        step(&mut rt, 2, 0.05);
        let ui = look(&rt, 2);
        assert!(close(shown(&ui, red()), 0.75), "{}", shown(&ui, red()));
        assert!(close(shown(&ui, blue()), 0.25), "{}", shown(&ui, blue()));

        step(&mut rt, 2, 0.05);
        let ui = look(&rt, 2);
        assert!(close(shown(&ui, red()), 0.5));
        assert!(close(shown(&ui, blue()), 0.5));

        step(&mut rt, 2, 0.1);
        let ui = look(&rt, 2);
        assert_eq!(shown(&ui, red()), 0.0, "gone from the tree");
        assert_eq!(shown(&ui, blue()), 1.0);
    }

    /// **The curves are the caller's**, one each way: the arriving child on the in-curve,
    /// the leaving one on the out-curve, read as its progress falls.
    #[test]
    fn each_direction_moves_on_its_own_curve() {
        let curved = |value: u8| {
            Flex::<i32>::row().child(
                AnimatedSwitcher::new(0.2, value, |v: &u8| tile(*v))
                    .switch_in_curve(Curve::Decelerate)
                    .switch_out_curve(Curve::Linear),
            )
        };
        let mut rt = Runtime::default();
        let tree = curved(1);
        let _ = build_ui(&tree, SIZE, &rt, &Theme::dark());
        rt.advance_switchers(&tree, 0.0);
        let tree = curved(2);
        let _ = build_ui(&tree, SIZE, &rt, &Theme::dark());
        rt.advance_switchers(&tree, 0.1);
        let ui = build_ui(&curved(2), SIZE, &rt, &Theme::dark());
        // Halfway: decelerating in is three quarters there, linear out is half gone.
        assert!(close(shown(&ui, blue()), 0.75), "{}", shown(&ui, blue()));
        assert!(close(shown(&ui, red()), 0.5), "{}", shown(&ui, red()));
    }

    /// **A child cut off on its way in leaves from where it had got to**, not from whole:
    /// the new value arrives while the last is still coming in, and the one that was
    /// arriving turns round at half rather than jumping to full and fading from there.
    #[test]
    fn a_child_cut_off_halfway_leaves_from_halfway() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);
        step(&mut rt, 2, 0.1); // blue half in
        step(&mut rt, 3, 0.0); // and now it leaves
        let ui = look(&rt, 3);
        assert!(close(shown(&ui, blue()), 0.5), "{}", shown(&ui, blue()));
        assert_eq!(shown(&ui, green()), 0.0, "the newest, not yet in");
        assert!(
            close(shown(&ui, red()), 0.5),
            "the first, still on its way out"
        );
    }

    /// **Equal values are the same child**: nothing moves, and nothing asks for the tree to
    /// be rebuilt.
    #[test]
    fn an_equal_value_is_not_a_switch() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);
        step(&mut rt, 1, 0.1);
        assert!(!rt.switching());
        assert_eq!(shown(&look(&rt, 1), red()), 1.0);
    }

    /// **While a switch moves, the frame has to be built, not only painted** — the progress
    /// is a number built into the children — and once it settles, it no longer does.
    #[test]
    fn a_switch_in_flight_asks_for_the_tree_to_be_built_and_then_stops() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);
        assert!(!rt.switching(), "at rest");
        step(&mut rt, 2, 0.05);
        assert!(rt.switching(), "moved since it was built");
        let _ = look(&rt, 2);
        assert!(!rt.switching(), "and built since");
        step(&mut rt, 2, 1.0);
        assert!(rt.switching(), "the step that ends it is a move too");
        step(&mut rt, 2, 0.05);
        assert!(!rt.switching(), "settled: painted only from here on");
    }

    /// **With motion turned down, it simply swaps**: the new child whole, the old one gone,
    /// on the frame the value changes.
    #[test]
    fn with_motion_turned_down_it_swaps_at_once() {
        let mut rt = Runtime::default();
        rt.still = true;
        step(&mut rt, 1, 0.0);
        let ui = look(&rt, 2);
        assert_eq!(shown(&ui, blue()), 1.0);
        assert_eq!(shown(&ui, red()), 0.0);
    }

    /// **The leaving child takes no input and is not announced.** The point is inside the
    /// wide red tile and outside the narrow blue one, so nothing but the red could answer
    /// it — and the red is a state the application has left.
    #[test]
    fn the_leaving_child_takes_no_input_and_is_not_announced() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);
        let before = look(&rt, 1);
        let only_red = Point::new(5.0, 10.0);
        let hit = before.hit(only_red).and_then(|id| before.msg_for(id));
        assert_eq!(
            hit,
            Some(1),
            "the fixture: the red tile answers there at rest"
        );

        step(&mut rt, 2, 0.1);
        let ui = look(&rt, 2);
        assert!(shown(&ui, red()) > 0.0, "the red is still on the screen");
        assert_eq!(ui.hit(only_red).and_then(|id| ui.msg_for(id)), None);
        let words: Vec<_> = ui
            .semantics()
            .iter()
            .filter_map(|(_, _, s)| s.label.clone())
            .collect();
        assert!(words.iter().any(|w| w.contains("two")), "{words:?}");
        assert!(!words.iter().any(|w| w.contains("one")), "{words:?}");
    }

    /// **While both show, the box is the larger of the two**, each centred in it; once the
    /// old one has gone, the box is the new one's.
    #[test]
    fn the_box_is_the_largest_child_while_both_show() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);
        step(&mut rt, 2, 0.1);
        let x_of = |ui: &Ui<i32>, color: Color| {
            rects(ui)
                .iter()
                .find(|(_, c, _)| *c == color)
                .map(|(r, _, _)| r.x)
                .unwrap()
        };
        let ui = look(&rt, 2);
        assert_eq!(x_of(&ui, red()), 0.0);
        assert_eq!(
            x_of(&ui, blue()),
            20.0,
            "the narrow one centred in the wide one's box"
        );

        step(&mut rt, 2, 1.0);
        assert_eq!(
            x_of(&look(&rt, 2), blue()),
            0.0,
            "and the box is its own again"
        );
    }

    /// **Behind a key, it still switches.** A key is a transparent wrapper: the walk hands
    /// it the identity it shares with the switcher, and it has to pass that on. Built
    /// through the older hook instead, the switcher never learns who it is and just shows
    /// the newest value whole.
    #[test]
    fn a_switcher_behind_a_key_still_switches() {
        let keyed = |value: u8| Flex::<i32>::row().child(Keyed::new(7u8, switcher(value)));
        let mut rt = Runtime::default();
        for (value, dt) in [(1, 0.0), (2, 0.1)] {
            let tree = keyed(value);
            let _ = build_ui(&tree, SIZE, &rt, &Theme::dark());
            rt.advance_switchers(&tree, dt);
        }
        let ui = build_ui(&keyed(2), SIZE, &rt, &Theme::dark());
        assert!(close(shown(&ui, red()), 0.5), "{}", shown(&ui, red()));
        assert!(close(shown(&ui, blue()), 0.5), "{}", shown(&ui, blue()));
    }

    /// **A switcher that goes is forgotten**, so one that appears in its place later is a
    /// mount — its value shown as it is — and not a switch from what used to be there.
    #[test]
    fn a_switcher_that_goes_is_forgotten() {
        let mut rt = Runtime::default();
        step(&mut rt, 1, 0.0);
        let without = Flex::<i32>::row().child(Container::new().width(10.0).height(10.0));
        let _ = build_ui(&without, SIZE, &rt, &Theme::dark());
        rt.advance_switchers(&without, 0.016);
        assert!(rt.switchers.borrow().is_empty(), "forgotten");

        step(&mut rt, 2, 0.0);
        let ui = look(&rt, 2);
        assert_eq!(shown(&ui, blue()), 1.0);
        assert_eq!(shown(&ui, red()), 0.0);
    }
}
