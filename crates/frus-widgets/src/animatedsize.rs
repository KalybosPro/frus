//! **`AnimatedSize`**: a box that follows its child's size over time rather than at once.
//!
//! When what it holds grows or shrinks — a paragraph that expands, a list that gains a row —
//! the box does not jump to the new size. It moves there over a duration, and the child is
//! clipped to it on the way, as the reference's is.
//!
//! The box is a measured one (milestone 587's plan for #52): its child is laid out in a tree
//! of its own, under the rule a [`ConstraintsTransformBox`](crate::ConstraintsTransformBox)
//! gives it, and the box answers with the size it has got to rather than the child's. The
//! runtime keeps, per box, where the size came from, where it is going and how far along it
//! is; the layout reads it and tells it when the child's size has changed.

use std::collections::{HashMap, HashSet};

use frus_core::animation::Curve;
use frus_core::{Alignment, Rect, Scene, Size};
use frus_layout::Style;

use crate::constraints::{AxisConstraint, ConstraintsTransform};
use crate::interaction::{Status, WidgetId};
use crate::theme::Theme;
use crate::widget::Widget;

/// How an [`AnimatedSize`] moves: over how long, along which curve, where the child sits in
/// the box on the way, and whether what spills out of it is cut.
#[derive(Clone, Debug)]
pub struct SizeAnimation {
    /// Seconds from one size to the next.
    pub duration: f32,
    /// The easing of that move.
    pub curve: Curve,
    /// Where the child sits in a box that is not its size yet.
    pub alignment: Alignment,
    /// Whether the child is cut to the box while it does not fit. On by default, as the
    /// reference's `Clip.hardEdge`.
    pub clip: bool,
}

/// One box's move: from, to, and how far along.
#[derive(Clone, Debug)]
struct Moving {
    from: Size,
    to: Size,
    elapsed: f32,
    duration: f32,
    curve: Curve,
    /// The child's size as this frame's layout last measured it, and how to move to it:
    /// taken up when the runtime next advances (milestone 609).
    seen: Option<(Size, SizeAnimation)>,
    /// First seen in the frame not yet advanced: it takes its child's size at once.
    fresh: bool,
}

impl Moving {
    fn settled(size: Size) -> Self {
        Self {
            from: size,
            to: size,
            elapsed: 0.0,
            duration: 0.0,
            curve: Curve::Linear,
            seen: None,
            fresh: false,
        }
    }

    fn done(&self) -> bool {
        self.elapsed >= self.duration
    }

    fn current(&self) -> Size {
        if self.done() {
            return self.to;
        }
        let t = self
            .curve
            .transform((self.elapsed / self.duration).clamp(0.0, 1.0));
        Size::new(
            self.from.width + (self.to.width - self.from.width) * t,
            self.from.height + (self.to.height - self.from.height) * t,
        )
    }
}

/// Every animated box's move, kept by the runtime between frames (milestone 594).
#[derive(Default, Debug)]
pub(crate) struct SizeAnims {
    moving: HashMap<WidgetId, Moving>,
    /// The boxes seen this frame, so that a box that has gone is forgotten.
    seen: HashSet<WidgetId>,
}

/// Two sizes the same to within a hundredth of a pixel: a child measured again at the same
/// size is not a change, whatever the last bit of the arithmetic says.
fn same(a: Size, b: Size) -> bool {
    (a.width - b.width).abs() < 0.01 && (a.height - b.height).abs() < 0.01
}

impl SizeAnims {
    /// The layout measured the child of `id` at `child`. Returns the size the box has now.
    ///
    /// **Noted, not acted on.** A layout asks the same child several questions in one pass
    /// — how wide with this much room, with that much, at its widest — and only the last is
    /// the size it ends up at. Each answer used to be taken as a new target, so a box whose
    /// child is given the width on offer saw its target move back and forth inside every
    /// frame, started over every frame, and never settled: the demonstration's home screen
    /// drew sixty frames a second doing nothing (milestone 609). The answer is kept, the
    /// last one winning, and taken up once, when the runtime advances: a change starts the
    /// move on the frame after it, as it did.
    ///
    /// A box seen for the first time takes its child's size at once, whichever answer was
    /// the last. `record` is false for the layout's intrinsic questions — how big with no
    /// room at all — which are not the size the child ends up at either.
    pub(crate) fn observe(
        &mut self,
        id: WidgetId,
        child: Size,
        spec: &SizeAnimation,
        record: bool,
    ) -> Size {
        self.seen.insert(id);
        match self.moving.get_mut(&id) {
            None => {
                if record {
                    let mut fresh = Moving::settled(child);
                    fresh.fresh = true;
                    self.moving.insert(id, fresh);
                }
                child
            }
            Some(moving) => {
                if record {
                    if moving.fresh {
                        // Still the frame it appeared in: it is the size it is.
                        moving.from = child;
                        moving.to = child;
                    } else {
                        moving.seen = Some((child, spec.clone()));
                    }
                }
                moving.current()
            }
        }
    }

    /// Takes up what the last layout measured: a box whose child has changed size starts
    /// over from wherever it had got to.
    fn take_up(moving: &mut Moving) {
        moving.fresh = false;
        let Some((child, spec)) = moving.seen.take() else {
            return;
        };
        if !same(moving.to, child) {
            *moving = Moving {
                from: moving.current(),
                to: child,
                elapsed: 0.0,
                duration: spec.duration.max(0.0),
                curve: spec.curve,
                seen: None,
                fresh: false,
            };
        }
    }

    /// Where the box of `id` is and where it is going, for the layout's fingerprint: a
    /// box on the move is a different geometry every frame, a box at rest is the same one.
    /// Marks the box as still there.
    pub(crate) fn fingerprint(&mut self, id: WidgetId) -> Option<(Size, Size)> {
        self.seen.insert(id);
        self.moving.get(&id).map(|m| (m.current(), m.to))
    }

    /// The size the child of `id` was last measured at: where the box is going.
    #[cfg(test)]
    fn target(&self, id: WidgetId) -> Option<Size> {
        self.moving.get(&id).map(|m| m.to)
    }

    /// Advances every move by `dt` seconds, or ends them all at once when motion is to be
    /// reduced. Forgets the boxes no frame has seen since the last call. Returns `true`
    /// while any is still moving.
    pub(crate) fn advance(&mut self, dt: f32, still: bool) -> bool {
        let seen = std::mem::take(&mut self.seen);
        self.moving.retain(|id, _| seen.contains(id));
        let mut animating = false;
        for moving in self.moving.values_mut() {
            Self::take_up(moving);
            if moving.done() {
                continue;
            }
            moving.elapsed = if still {
                moving.duration
            } else {
                moving.elapsed + dt
            };
            animating |= !moving.done();
        }
        animating
    }
}

/// A box that **follows its child's size over time**: when the child grows or shrinks, the
/// box moves to the new size over `duration` seconds instead of jumping there, and the child
/// is clipped to it on the way.
///
/// By default the child is given the width on offer and asked for its height, which is what
/// a section that expands and collapses wants. [`width`](Self::width) and
/// [`height`](Self::height) change that, in the vocabulary of a
/// [`ConstraintsTransformBox`](crate::ConstraintsTransformBox).
///
/// ```
/// use frus_widgets::{text, AnimatedSize, Flex};
/// # let expanded = true;
/// let mut body = Flex::<()>::column().child(text("Summary"));
/// if expanded {
///     body = body.child(text("And everything else, which only shows when asked."));
/// }
/// let section = AnimatedSize::new(0.2, body);
/// ```
///
/// A change is seen by the layout, so the box starts moving on the frame after the child
/// changed; the reference's starts on the same frame.
pub struct AnimatedSize<Msg = crate::callback::Callback> {
    transform: ConstraintsTransform,
    spec: SizeAnimation,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> AnimatedSize<Msg> {
    /// A box holding `child`, following its size over `duration` seconds, linearly, with
    /// the child centred while it does not fit.
    pub fn new(duration: f32, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            transform: ConstraintsTransform {
                width: AxisConstraint::AsGiven,
                height: AxisConstraint::Unbounded,
                alignment: Alignment::CENTER,
                report: false,
            },
            spec: SizeAnimation {
                duration,
                curve: Curve::Linear,
                alignment: Alignment::CENTER,
                clip: true,
            },
            children: vec![Box::new(child)],
        }
    }

    /// The easing of the move.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.spec.curve = curve;
        self
    }

    /// Where the child sits in a box that is not its size yet.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.spec.alignment = alignment;
        self.transform.alignment = alignment;
        self
    }

    /// Whether the child is cut to the box while it does not fit. On by default.
    pub fn clip(mut self, clip: bool) -> Self {
        self.spec.clip = clip;
        self
    }

    /// What the child is given across: the width on offer by default.
    pub fn width(mut self, width: AxisConstraint) -> Self {
        self.transform.width = width;
        self
    }

    /// What the child is given down: nothing by default, so it is asked for its height.
    pub fn height(mut self, height: AxisConstraint) -> Self {
        self.transform.height = height;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for AnimatedSize<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn constraints_transform(&self) -> Option<ConstraintsTransform> {
        Some(self.transform)
    }

    fn animated_size(&self) -> Option<SizeAnimation> {
        Some(self.spec.clone())
    }

    /// The room on offer, on an axis the child is given as it came: that axis is not the
    /// child's to decide, so the box is as big as the room there, as the reference's is under
    /// tight constraints.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: self.transform.width == AxisConstraint::AsGiven,
            vertical: self.transform.height == AxisConstraint::AsGiven,
        }
    }

    fn debug_name(&self) -> &'static str {
        "AnimatedSize"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Flex, Runtime};

    fn spec(duration: f32) -> SizeAnimation {
        SizeAnimation {
            duration,
            curve: Curve::Linear,
            alignment: Alignment::CENTER,
            clip: true,
        }
    }

    const ID: WidgetId = WidgetId::ROOT;

    /// A box seen for the first time is its child's size at once: nothing to move from.
    #[test]
    fn a_new_box_starts_at_its_childs_size() {
        let mut anims = SizeAnims::default();
        let shown = anims.observe(ID, Size::new(100.0, 40.0), &spec(0.2), true);
        assert_eq!(shown, Size::new(100.0, 40.0));
        assert!(!anims.advance(0.016, false), "nothing is moving");
    }

    /// A child that grows is followed, not jumped to: the box starts where it was and gets
    /// there after the duration, half-way at half the time.
    #[test]
    fn a_grown_child_is_followed_over_the_duration() {
        let mut anims = SizeAnims::default();
        anims.observe(ID, Size::new(100.0, 40.0), &spec(0.2), true);
        // The next frame, as the shell advances between two.
        anims.advance(0.0, false);
        let now = anims.observe(ID, Size::new(100.0, 140.0), &spec(0.2), true);
        assert_eq!(now.height, 40.0, "it starts where it was");
        assert!(anims.advance(0.1, false));
        let half = anims.observe(ID, Size::new(100.0, 140.0), &spec(0.2), true);
        assert!((half.height - 90.0).abs() < 0.01, "{half:?}");
        assert!(!anims.advance(0.1, false));
        assert_eq!(
            anims.observe(ID, Size::new(100.0, 140.0), &spec(0.2), true),
            Size::new(100.0, 140.0)
        );
    }

    /// A change in the middle of a move starts from wherever the box had got to.
    #[test]
    fn a_change_mid_move_starts_from_where_it_is() {
        let mut anims = SizeAnims::default();
        anims.observe(ID, Size::new(100.0, 0.0), &spec(0.2), true);
        anims.advance(0.0, false);
        anims.observe(ID, Size::new(100.0, 200.0), &spec(0.2), true);
        anims.advance(0.1, false);
        let back = anims.observe(ID, Size::new(100.0, 0.0), &spec(0.2), true);
        assert!((back.height - 100.0).abs() < 0.01, "{back:?}");
        anims.advance(0.1, false);
        let later = anims.observe(ID, Size::new(100.0, 0.0), &spec(0.2), true);
        assert!((later.height - 50.0).abs() < 0.01, "{later:?}");
    }

    /// **One frame's questions are not a change** (milestone 609): a layout measures the
    /// same child at several widths in one pass, and only the last answer is the size it
    /// ends up at. A box asked about 297, 481 and 312 px in one frame, and about the same in
    /// the next, has not moved, and asks for no more frames.
    #[test]
    fn the_questions_of_one_frame_are_not_a_change() {
        let mut anims = SizeAnims::default();
        let asked = |anims: &mut SizeAnims| {
            for width in [297.0, 481.0, 312.0] {
                anims.observe(ID, Size::new(width, 22.0), &spec(0.2), true);
            }
        };
        asked(&mut anims);
        assert!(!anims.advance(0.016, false), "it appeared at its size");
        for _ in 0..3 {
            asked(&mut anims);
            assert!(
                !anims.advance(0.016, false),
                "the same answers: nothing to follow"
            );
        }
        // And a real change, the last answer, still starts a move.
        for width in [297.0, 481.0, 350.0] {
            anims.observe(ID, Size::new(width, 22.0), &spec(0.2), true);
        }
        assert!(anims.advance(0.016, false), "a new last answer is followed");
    }

    /// The layout's intrinsic questions are answered and not taken for a change.
    #[test]
    fn a_probe_is_not_a_change() {
        let mut anims = SizeAnims::default();
        anims.observe(ID, Size::new(100.0, 40.0), &spec(0.2), true);
        let probe = anims.observe(ID, Size::new(0.0, 900.0), &spec(0.2), false);
        assert_eq!(probe, Size::new(100.0, 40.0));
        assert!(!anims.advance(0.016, false));
    }

    /// Reduced motion ends every move at once.
    #[test]
    fn reduced_motion_arrives_at_once() {
        let mut anims = SizeAnims::default();
        anims.observe(ID, Size::new(100.0, 40.0), &spec(0.2), true);
        anims.observe(ID, Size::new(100.0, 140.0), &spec(0.2), true);
        assert!(!anims.advance(0.001, true));
        assert_eq!(
            anims.observe(ID, Size::new(100.0, 140.0), &spec(0.2), true),
            Size::new(100.0, 140.0)
        );
    }

    /// A box no frame has seen is forgotten.
    #[test]
    fn a_box_that_has_gone_is_forgotten() {
        let mut anims = SizeAnims::default();
        anims.observe(ID, Size::new(100.0, 40.0), &spec(0.2), true);
        anims.advance(0.016, false);
        assert!(anims.target(ID).is_some());
        anims.advance(0.016, false);
        assert!(anims.target(ID).is_none());
    }

    fn page(rows: usize) -> Flex<()> {
        let mut body = Flex::<()>::column();
        for _ in 0..rows {
            body = body.child(Container::<()>::new().height(40.0));
        }
        Flex::<()>::column()
            .child(AnimatedSize::new(0.2, body))
            .child(Container::<()>::new().height(10.0))
    }

    /// The box's height as the layout gives it, and where the thing below it starts.
    fn heights(rt: &Runtime, rows: usize) -> (f32, f32) {
        let tree = page(rows);
        let ui = build_ui(&tree, Size::new(300.0, 600.0), rt, &Theme::default());
        let _ = ui;
        let (rects, _) = rt.layout_cache.borrow_mut().rects(
            WidgetId::ROOT,
            &tree,
            rt,
            &Theme::default(),
            crate::relayout::Constraints::definite(Size::new(300.0, 600.0)),
        );
        (rects[1].height, rects[2].y)
    }

    /// **In a page**: one row, then three. The box is one row tall, still one row tall on
    /// the frame the change is seen, part of the way on the frames after, and three rows tall
    /// once the duration is over — and what is below it moves down with it.
    #[test]
    fn a_page_follows_a_growing_section() {
        let mut rt = Runtime::default();
        assert_eq!(heights(&rt, 1), (40.0, 40.0));
        rt.advance(0.0);
        assert_eq!(heights(&rt, 3).0, 40.0, "the frame the change is seen");
        assert!(rt.advance(0.1));
        let (half, below) = heights(&rt, 3);
        assert!((half - 80.0).abs() < 0.5, "half-way: {half}");
        assert!(
            (below - half).abs() < 0.5,
            "the next thing follows: {below}"
        );
        while rt.advance(0.05) {
            heights(&rt, 3);
        }
        assert_eq!(heights(&rt, 3), (120.0, 120.0));
    }

    /// **An axis the child is given is the room's, at once**: a container widened from 200
    /// to 300 px, and the bar inside, which fills the width it is given, is 300 px on the
    /// next frame. The width is not the child's to decide, so there is nothing to follow; the
    /// reference's box, under tight constraints, is not animated either.
    #[test]
    fn the_child_is_laid_out_at_its_new_size_at_once() {
        let page = |width: f32| {
            Container::<()>::new().width(width).child(
                AnimatedSize::new(
                    0.2,
                    Container::<()>::new()
                        .height(20.0)
                        .color(frus_core::Color::rgb(0.2, 0.4, 0.8)),
                )
                .alignment(Alignment::TOP_LEFT),
            )
        };
        let bar = |ui: &crate::Ui<()>| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { rect, .. } => Some(*rect),
                    _ => None,
                })
                .expect("the bar")
        };
        let mut rt = Runtime::default();
        let size = Size::new(400.0, 300.0);
        assert_eq!(
            bar(&build_ui(&page(200.0), size, &rt, &Theme::default())).width,
            200.0
        );
        build_ui(&page(300.0), size, &rt, &Theme::default());
        rt.advance(0.1);
        let ui = build_ui(&page(300.0), size, &rt, &Theme::default());
        assert_eq!(bar(&ui).width, 300.0, "laid out at where it is going");
    }

    /// **The layout's intrinsic questions are not a change of size.** In a row, the layout
    /// asks the box how narrow it could be before it gives it its width, and a paragraph
    /// given no width at all is as tall as all its words stacked. The box must not start from
    /// that: at rest from the first frame, as tall as the paragraph at the width it gets.
    #[test]
    fn a_row_asking_how_narrow_does_not_start_a_move() {
        let words = "Some words that wrap once they are given a width to wrap in.";
        let tree = crate::Row::<()>::new().child(AnimatedSize::new(0.2, crate::Text::new(words)));
        let mut rt = Runtime::default();
        let size = Size::new(300.0, 600.0);
        let first = build_ui(&tree, size, &rt, &Theme::default());
        let _ = first;
        let (rects, _) = rt.layout_cache.borrow_mut().rects(
            WidgetId::ROOT,
            &tree,
            &rt,
            &Theme::default(),
            crate::relayout::Constraints::definite(size),
        );
        let settled = rects[1].height;
        assert!(
            settled < 100.0,
            "the paragraph at the width it gets: {settled}"
        );
        assert!(!rt.advance(0.05), "nothing to follow");
    }

    /// **On the way, the child is cut to the box**: half-way from one row to three, the rows
    /// are painted at their own places and nothing shows below the box's 80 px.
    #[test]
    fn the_child_is_cut_to_the_box_on_the_way() {
        let coloured = |rows: usize| {
            let mut body = Flex::<()>::column();
            for _ in 0..rows {
                body = body.child(
                    Container::<()>::new()
                        .height(40.0)
                        .color(frus_core::Color::rgb(0.2, 0.4, 0.8)),
                );
            }
            Flex::<()>::column().child(AnimatedSize::new(0.2, body).alignment(Alignment::TOP_LEFT))
        };
        let mut rt = Runtime::default();
        let size = Size::new(300.0, 600.0);
        build_ui(&coloured(1), size, &rt, &Theme::default());
        rt.advance(0.0);
        build_ui(&coloured(3), size, &rt, &Theme::default());
        rt.advance(0.1);
        let ui = build_ui(&coloured(3), size, &rt, &Theme::default());
        let painted: Vec<Rect> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { rect, clip, .. } => Some(rect.intersect(*clip)),
                _ => None,
            })
            .filter(|r| r.width > 0.0 && r.height > 0.0)
            .collect();
        assert_eq!(
            painted.len(),
            2,
            "two rows show, the third is cut away: {painted:?}"
        );
        let bottom = painted.iter().map(|r| r.y + r.height).fold(0.0, f32::max);
        assert!((bottom - 80.0).abs() < 0.5, "cut at the box: {painted:?}");
    }
}
