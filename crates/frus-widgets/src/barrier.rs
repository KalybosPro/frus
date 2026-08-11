//! Widgets that **withhold** part of the frame: [`IgnorePointer`] and
//! [`AbsorbPointer`] take a subtree out of the input path, [`Visibility`] and
//! [`Offstage`] take it out of the picture, [`ExcludeSemantics`] takes it out of the
//! accessibility tree.
//!
//! They all express the same idea — *this subtree is built, but part of what it
//! produced does not count* — and they share one mechanism, [`Barrier`], applied by the
//! walk in `ui.rs`: the subtree is walked normally, then whatever it added to the
//! selected registries is dropped again.
//!
//! Dropping afterwards rather than skipping beforehand is what makes them **exact**.
//! A widget deep inside can register a click target, a focus stop, a scrollable area,
//! a drag handle or an accessibility node, and it does so without knowing that
//! something above it is holding the whole subtree out of the frame. Removing at the
//! barrier catches every one of those, including the ones added by widgets written
//! later.
//!
//! ## Which to reach for
//!
//! | | laid out | painted | takes input | blocks what is behind |
//! |---|---|---|---|---|
//! | [`IgnorePointer`] | yes | yes | no | no — input falls through |
//! | [`AbsorbPointer`] | yes | yes | no | yes — input stops here |
//! | [`Visibility`] hidden, keeping its size | yes | no | no | no |
//! | [`Visibility`] hidden | no | no | no | no |
//! | [`Offstage`] | no | no | no | no |
//! | [`ExcludeSemantics`] | yes | yes | yes | n/a |
//!
//! The difference between the first two is the one that matters and is easy to get
//! backwards: a disabled control that should let a click reach the card underneath is
//! an `IgnorePointer`; a shield over a screen that is busy, which must swallow every
//! stray tap, is an `AbsorbPointer`.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// What a subtree is not allowed to contribute to the frame.
///
/// Returned by [`Widget::barrier`]; the walk applies it once the subtree has been
/// visited, by discarding what that subtree added.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Barrier {
    /// Drops the subtree's input targets: clicks, long presses, focus stops,
    /// scrollable areas, scrollbars, drag handles, reorder handles and interactive
    /// viewports.
    pub pointer: bool,
    /// Makes the barrier itself an input target that carries no message, so input
    /// landing on it stops there instead of reaching whatever is painted behind. Only
    /// meaningful together with `pointer`.
    pub absorb: bool,
    /// Drops the subtree's primitives — laid out and measured, but not drawn.
    pub paint: bool,
    /// Drops the subtree's accessibility nodes.
    pub semantics: bool,
}

impl Barrier {
    /// A barrier that lets everything through — what a widget with no opinion returns.
    pub const NONE: Self = Self {
        pointer: false,
        absorb: false,
        paint: false,
        semantics: false,
    };

    /// `true` when this barrier would change nothing, so the walk can skip it.
    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }
}

/// The style of a box that takes no room at all.
fn collapsed() -> Style {
    Style {
        width: Dimension::Length(0.0),
        height: Dimension::Length(0.0),
        ..Default::default()
    }
}

/// Makes its child **invisible to input** while leaving it laid out and painted.
///
/// Input passes straight through to whatever is behind — which is the whole difference
/// from [`AbsorbPointer`].
///
/// ```ignore
/// IgnorePointer::new(row).ignoring(saving)   // inert while a save is in flight
/// ```
///
/// The subtree keeps its accessibility nodes: a screen reader can still read a form
/// that is momentarily inert, which is more useful than having it vanish. Wrap it in
/// [`ExcludeSemantics`] as well if it really should be unreachable.
pub struct IgnorePointer<Msg> {
    ignoring: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> IgnorePointer<Msg> {
    /// Takes `child` out of the input path.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            ignoring: true,
            children: vec![Box::new(child)],
        }
    }

    /// Turns the barrier on or off from a condition — `false` leaves the child fully
    /// interactive, so a caller does not have to build two different trees.
    pub fn ignoring(mut self, ignoring: bool) -> Self {
        self.ignoring = ignoring;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for IgnorePointer<Msg> {
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

    fn barrier(&self) -> Option<Barrier> {
        self.ignoring.then_some(Barrier {
            pointer: true,
            ..Barrier::NONE
        })
    }
}

/// Makes its child invisible to input **and stops there**: the barrier is itself an
/// input target, so anything landing on it is swallowed rather than reaching what is
/// painted behind.
///
/// This is the shield over a screen that is busy, and the backdrop of a modal.
///
/// ```ignore
/// AbsorbPointer::new(screen).absorbing(loading)
/// ```
pub struct AbsorbPointer<Msg> {
    absorbing: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> AbsorbPointer<Msg> {
    /// Takes `child` out of the input path and swallows what lands on it.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            absorbing: true,
            children: vec![Box::new(child)],
        }
    }

    /// Turns the barrier on or off from a condition.
    pub fn absorbing(mut self, absorbing: bool) -> Self {
        self.absorbing = absorbing;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for AbsorbPointer<Msg> {
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

    fn barrier(&self) -> Option<Barrier> {
        self.absorbing.then_some(Barrier {
            pointer: true,
            absorb: true,
            ..Barrier::NONE
        })
    }
}

/// Hides its child from assistive technologies while leaving it visible and
/// interactive — for decoration that would only add noise to a screen reader, or for a
/// label the control beside it already announces.
///
/// ```ignore
/// ExcludeSemantics::new(Icon::new(icons::CHEVRON))   // the button already says "next"
/// ```
pub struct ExcludeSemantics<Msg> {
    excluding: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> ExcludeSemantics<Msg> {
    /// Removes `child`'s subtree from the accessibility tree.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            excluding: true,
            children: vec![Box::new(child)],
        }
    }

    /// Turns the exclusion on or off from a condition.
    pub fn excluding(mut self, excluding: bool) -> Self {
        self.excluding = excluding;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ExcludeSemantics<Msg> {
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

    fn barrier(&self) -> Option<Barrier> {
        self.excluding.then_some(Barrier {
            semantics: true,
            ..Barrier::NONE
        })
    }
}

/// Takes its child **out of the layout entirely**: no box, no paint, no input, no
/// accessibility. The surrounding row or column closes up as if the child were not
/// written at all.
///
/// ```ignore
/// Offstage::new(panel).offstage(!expanded)
/// ```
///
/// The child is not built into the tree while offstage, so it holds no retained state
/// — a caret position, a scroll offset — and starts fresh when it comes back. That is
/// the right trade for a branch that is genuinely gone; [`Visibility::maintain_size`]
/// is the one to reach for when the box has to stay.
pub struct Offstage<Msg> {
    offstage: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Offstage<Msg> {
    /// Removes `child` from the layout.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            offstage: true,
            children: vec![Box::new(child)],
        }
    }

    /// Turns it on or off from a condition; `false` lays the child out normally.
    pub fn offstage(mut self, offstage: bool) -> Self {
        self.offstage = offstage;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Offstage<Msg> {
    fn style(&self) -> Style {
        if self.offstage {
            collapsed()
        } else {
            Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        if self.offstage {
            &[]
        } else {
            &self.children
        }
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Shows or hides its child, with a say in **how much of it survives** being hidden.
///
/// The plain form collapses the child away entirely, as [`Offstage`] does:
///
/// ```ignore
/// Visibility::new(banner).visible(has_error)
/// ```
///
/// `maintain_size` keeps the box — the child still measures and still takes its space,
/// it is simply not drawn. That is what stops a row from twitching as a spinner comes
/// and goes, and what lets two alternating labels of different widths hold a column
/// steady:
///
/// ```ignore
/// Visibility::new(spinner).visible(loading).maintain_size()
/// ```
///
/// `replacement` puts something else in the collapsed child's place instead — a
/// spacer, a placeholder, a shorter message.
pub struct Visibility<Msg> {
    visible: bool,
    maintain_size: bool,
    maintain_interactivity: bool,
    maintain_semantics: bool,
    /// At most one entry. A `Vec` rather than an `Option` so `children` can hand out a
    /// slice without building one.
    child: Vec<Box<dyn Widget<Msg>>>,
    replacement: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Visibility<Msg> {
    /// Shows `child`. Combine with [`visible`](Self::visible) to make it conditional.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            visible: true,
            maintain_size: false,
            maintain_interactivity: false,
            maintain_semantics: false,
            child: vec![Box::new(child)],
            replacement: Vec::new(),
        }
    }

    /// Whether the child is shown.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Keeps the child's **box** while hidden: it is laid out and measured as usual,
    /// but not drawn, not clickable and not read out.
    pub fn maintain_size(mut self) -> Self {
        self.maintain_size = true;
        self
    }

    /// Keeps the child **taking input** while hidden. Only meaningful together with
    /// [`maintain_size`](Self::maintain_size), and rarely what you want: an invisible
    /// control that still reacts surprises everyone but its author.
    pub fn maintain_interactivity(mut self) -> Self {
        self.maintain_interactivity = true;
        self
    }

    /// Keeps the child in the **accessibility tree** while hidden — for content that
    /// is visually redundant but should still be announced.
    pub fn maintain_semantics(mut self) -> Self {
        self.maintain_semantics = true;
        self
    }

    /// What to show in the child's place when it is hidden **and** its box is not
    /// maintained.
    pub fn replacement(mut self, replacement: impl Widget<Msg> + 'static) -> Self {
        self.replacement = vec![Box::new(replacement)];
        self
    }

    /// `true` when the child is in the tree this frame (shown, or hidden but still
    /// occupying its box).
    fn keeps_child(&self) -> bool {
        self.visible || self.maintain_size
    }
}

impl<Msg: Clone> Widget<Msg> for Visibility<Msg> {
    fn style(&self) -> Style {
        if self.keeps_child() || !self.replacement.is_empty() {
            Style::default()
        } else {
            collapsed()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        if self.keeps_child() {
            &self.child
        } else {
            &self.replacement
        }
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn barrier(&self) -> Option<Barrier> {
        // Only the "hidden but still occupying its box" case needs a barrier: the
        // collapsed case has no child in the tree to withhold anything.
        if self.visible || !self.maintain_size {
            return None;
        }
        Some(Barrier {
            pointer: !self.maintain_interactivity,
            absorb: false,
            paint: true,
            semantics: !self.maintain_semantics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Button, Container, Runtime, Scroll, Text};
    use frus_core::{Color, Point, Primitive, Size};

    fn ui_of(root: &dyn Widget<i32>) -> crate::Ui<i32> {
        build_ui(
            root,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &crate::Theme::dark(),
        )
    }

    /// A 100 × 100 red box emitting `msg` when clicked.
    fn button(msg: i32) -> Container<i32> {
        Container::new()
            .width(100.0)
            .height(100.0)
            .color(Color::rgb(1.0, 0.0, 0.0))
            .on_click(msg)
    }

    /// The centre of the surface — inside every box these tests build.
    fn centre() -> Point {
        Point::new(50.0, 50.0)
    }

    #[test]
    fn ignoring_takes_the_subtree_out_of_the_input_path() {
        let plain =
            crate::Flex::<i32>::column().child(IgnorePointer::new(button(1)).ignoring(false));
        let ui = ui_of(&plain);
        assert_eq!(
            ui.hit(centre()).and_then(|id| ui.msg_for(id)),
            Some(1),
            "ignoring(false) leaves the child fully interactive"
        );

        let ignored = crate::Flex::<i32>::column().child(IgnorePointer::new(button(1)));
        let ui = ui_of(&ignored);
        assert!(ui.hit(centre()).is_none());
    }

    #[test]
    fn ignoring_lets_input_reach_what_is_behind() {
        // Two overlapping layers: the ignored one is painted on top.
        let root = crate::Stack::<i32>::new()
            .width(100.0)
            .height(100.0)
            .layer(button(1))
            .layer(IgnorePointer::new(button(2)));
        let ui = ui_of(&root);
        assert_eq!(
            ui.hit(centre()).and_then(|id| ui.msg_for(id)),
            Some(1),
            "the click falls through the ignored layer to the one underneath"
        );
    }

    #[test]
    fn absorbing_stops_input_at_the_barrier() {
        let root = crate::Stack::<i32>::new()
            .width(100.0)
            .height(100.0)
            .layer(button(1))
            .layer(AbsorbPointer::new(button(2)));
        let ui = ui_of(&root);
        let id = ui.hit(centre()).expect("the barrier is itself a target");
        assert_eq!(
            ui.msg_for(id),
            None,
            "it swallows the click rather than emitting anything"
        );
    }

    #[test]
    fn absorbing_can_be_turned_off_from_a_condition() {
        let root = crate::Stack::<i32>::new()
            .width(100.0)
            .height(100.0)
            .layer(button(1))
            .layer(AbsorbPointer::new(button(2)).absorbing(false));
        let ui = ui_of(&root);
        assert_eq!(ui.hit(centre()).and_then(|id| ui.msg_for(id)), Some(2));
    }

    #[test]
    fn a_barrier_still_paints_its_child() {
        let root = crate::Flex::<i32>::column().child(IgnorePointer::new(button(1)));
        let ui = ui_of(&root);
        assert!(
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Rect { color, .. } if color.r > 0.5)),
            "an ignored subtree is invisible to input, not to the eye"
        );
    }

    /// The barrier acts on what the subtree registered, not on what the barrier widget
    /// itself declares — so a target several levels down is caught too.
    #[test]
    fn a_target_deep_inside_is_caught() {
        let deep = crate::Flex::<i32>::column().child(
            crate::Flex::<i32>::column().child(crate::Flex::<i32>::column().child(button(1))),
        );
        let root = crate::Flex::<i32>::column().child(IgnorePointer::new(deep));
        let ui = ui_of(&root);
        assert!(ui.hit(centre()).is_none());
    }

    #[test]
    fn a_scrollable_inside_a_barrier_stops_scrolling() {
        let long = Scroll::<i32>::new()
            .width(100.0)
            .height(100.0)
            .child(Container::new().width(100.0).height(1000.0));
        let root = crate::Flex::<i32>::column().child(IgnorePointer::new(long));
        let ui = ui_of(&root);
        assert!(
            ui.scroll_hit(centre()).is_none(),
            "the wheel has nothing to scroll through a barrier"
        );
    }

    /// A widget that annotates itself for assistive technologies — the only kind
    /// [`ExcludeSemantics`] has anything to remove.
    fn announced() -> Button<i32> {
        Button::new("Send").on_press(1)
    }

    #[test]
    fn excluding_semantics_leaves_input_and_paint_alone() {
        let visible = crate::Flex::<i32>::column().child(announced());
        let plain = ui_of(&visible);
        assert!(
            !plain.semantics().is_empty(),
            "the fixture has something to exclude"
        );
        // A point inside the button, which sits at the top of the column.
        let on_the_button = Point::new(20.0, 12.0);
        assert!(plain.hit(on_the_button).is_some(), "and it is clickable");

        let root = crate::Flex::<i32>::column().child(ExcludeSemantics::new(announced()));
        let ui = ui_of(&root);
        assert!(
            ui.semantics().is_empty(),
            "gone from the accessibility tree"
        );
        assert!(ui.hit(on_the_button).is_some(), "and still clickable");
    }

    #[test]
    fn an_ignored_subtree_keeps_its_semantics() {
        let root = crate::Flex::<i32>::column().child(IgnorePointer::new(announced()));
        let ui = ui_of(&root);
        assert!(
            !ui.semantics().is_empty(),
            "a momentarily inert form is still worth reading out"
        );
        assert!(
            ui.hit(Point::new(20.0, 12.0)).is_none(),
            "but it takes no input"
        );
    }

    #[test]
    fn offstage_takes_no_room() {
        let shown = crate::Flex::<i32>::column()
            .child(Offstage::new(button(1)).offstage(false))
            .child(Text::new("after"));
        let ui = ui_of(&shown);
        assert!(ui.hit(centre()).is_some());

        let hidden = crate::Flex::<i32>::column()
            .child(Offstage::new(button(1)))
            .child(Text::new("after"));
        let ui = ui_of(&hidden);
        assert!(ui.hit(centre()).is_none());
        assert!(
            ui.scene()
                .primitives()
                .iter()
                .all(|p| !matches!(p, Primitive::Rect { color, .. } if color.r > 0.5)),
            "an offstage child is not painted either"
        );
    }

    #[test]
    fn a_hidden_visibility_collapses_by_default() {
        let root = crate::Flex::<i32>::column().child(Visibility::new(button(1)).visible(false));
        let ui = ui_of(&root);
        assert!(ui.hit(centre()).is_none());
        assert_eq!(
            Widget::<i32>::children(&Visibility::new(button(1)).visible(false)).len(),
            0,
            "no child in the tree at all"
        );
    }

    #[test]
    fn maintaining_the_size_keeps_the_box_but_not_the_paint() {
        let hidden = || Visibility::new(button(1)).visible(false).maintain_size();
        let root = crate::Flex::<i32>::column().child(hidden());
        let ui = ui_of(&root);
        assert!(
            ui.scene()
                .primitives()
                .iter()
                .all(|p| !matches!(p, Primitive::Rect { color, .. } if color.r > 0.5)),
            "not drawn"
        );
        assert!(ui.hit(centre()).is_none(), "and not clickable");
        assert_eq!(
            Widget::<i32>::children(&hidden()).len(),
            1,
            "but still laid out, so the row does not twitch"
        );
    }

    #[test]
    fn maintained_interactivity_survives_the_barrier() {
        let root = crate::Flex::<i32>::column().child(
            Visibility::new(button(1))
                .visible(false)
                .maintain_size()
                .maintain_interactivity(),
        );
        let ui = ui_of(&root);
        assert_eq!(ui.hit(centre()).and_then(|id| ui.msg_for(id)), Some(1));
    }

    #[test]
    fn a_replacement_takes_the_hidden_childs_place() {
        let root = crate::Flex::<i32>::column().child(
            Visibility::new(button(1))
                .visible(false)
                .replacement(button(2)),
        );
        let ui = ui_of(&root);
        assert_eq!(
            ui.hit(centre()).and_then(|id| ui.msg_for(id)),
            Some(2),
            "the replacement is an ordinary child, barrier-free"
        );
    }

    #[test]
    fn a_widget_that_withholds_nothing_declares_no_barrier() {
        assert_eq!(Widget::<i32>::barrier(&Visibility::new(button(1))), None);
        assert_eq!(
            Widget::<i32>::barrier(&IgnorePointer::new(button(1)).ignoring(false)),
            None
        );
    }
}
