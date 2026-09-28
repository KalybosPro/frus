//! **`AnimatedCrossFade`**: one of two children shown, the other faded in over it when the
//! choice changes, and the box moving from the first one's size to the second one's.
//!
//! It is composed, as the reference's is, of pieces that already exist: an
//! [`AnimatedSize`](crate::AnimatedSize) around a cell holding both children, each fading
//! towards shown or hidden. The child being shown is laid out in the cell and gives it its
//! size. The other is laid over it, at the cell's width and its own height, in a box that
//! counts for nothing — so that the size the box moves to is the shown child's alone — and
//! whatever of it runs past the box is cut.

use std::cell::{OnceCell, RefCell};

use frus_core::animation::Curve;
use frus_core::{Alignment, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Which child an [`AnimatedCrossFade`] shows.
const FIRST: u8 = 0;
const SECOND: u8 = 1;

/// Shows one of two children, and **cross-fades** to the other when told to, the box moving
/// between their sizes as they fade.
///
/// ```
/// use frus_widgets::{text, AnimatedCrossFade};
/// # let expanded = false;
/// let summary = AnimatedCrossFade::<()>::new(
///     text("Three tasks"),
///     text("Buy milk, write code, call Mum"),
///     !expanded,
///     0.25,
/// );
/// ```
///
/// The child that is not shown takes no taps and is not read out, and neither is the one on
/// its way out. Both children fill the width the cross-fade is given.
pub struct AnimatedCrossFade<Msg = crate::callback::Callback> {
    first: RefCell<Option<Box<dyn Widget<Msg>>>>,
    second: RefCell<Option<Box<dyn Widget<Msg>>>>,
    show_first: bool,
    duration: f32,
    first_curve: Curve,
    second_curve: Curve,
    size_curve: Curve,
    alignment: Alignment,
    /// The composition, made on the first walk so that the order the builders were called
    /// in cannot change it.
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> AnimatedCrossFade<Msg> {
    /// Shows `first` when `show_first`, `second` otherwise, cross-fading over `duration`
    /// seconds when that changes.
    pub fn new(
        first: impl Widget<Msg> + 'static,
        second: impl Widget<Msg> + 'static,
        show_first: bool,
        duration: f32,
    ) -> Self {
        Self {
            first: RefCell::new(Some(Box::new(first))),
            second: RefCell::new(Some(Box::new(second))),
            show_first,
            duration,
            first_curve: Curve::Linear,
            second_curve: Curve::Linear,
            size_curve: Curve::Linear,
            alignment: Alignment::TOP_CENTER,
            built: OnceCell::new(),
        }
    }

    /// The easing of the first child's fade.
    pub fn first_curve(mut self, curve: Curve) -> Self {
        self.first_curve = curve;
        self
    }

    /// The easing of the second child's fade.
    pub fn second_curve(mut self, curve: Curve) -> Self {
        self.second_curve = curve;
        self
    }

    /// The easing of the box's move between the two sizes.
    pub fn size_curve(mut self, curve: Curve) -> Self {
        self.size_curve = curve;
        self
    }

    /// Where the children sit in a box that is not their size yet: the top, centred, by
    /// default, as the reference's.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// One child's place in the cell: faded towards shown or hidden, taking taps and being
    /// read out only while shown, and — while hidden — laid over the cell rather than in it.
    ///
    /// The same four wrappers either way, under a key of its own, so that a child keeps its
    /// fade when it goes from shown to hidden and back.
    fn slot(&self, which: u8, child: Box<dyn Widget<Msg>>, curve: Curve) -> Box<dyn Widget<Msg>> {
        let shown = (which == FIRST) == self.show_first;
        let placed: Box<dyn Widget<Msg>> = if shown {
            child
        } else {
            Box::new(
                crate::OverflowBox::new(child)
                    .natural_height()
                    .alignment(self.alignment),
            )
        };
        let faded = crate::AnimatedOpacity::new(
            if shown { 1.0 } else { 0.0 },
            self.duration,
            curve,
            FillWidth {
                children: vec![placed],
            },
        );
        Box::new(crate::Keyed::new(
            ("cross-fade", which),
            crate::IgnorePointer::new(crate::ExcludeSemantics::new(faded).excluding(!shown))
                .ignoring(!shown),
        ))
    }

    fn assemble(&self) -> Vec<Box<dyn Widget<Msg>>> {
        let first = self.first.borrow_mut().take().expect("assembled once");
        let second = self.second.borrow_mut().take().expect("assembled once");
        let first = self.slot(FIRST, first, self.first_curve.clone());
        let second = self.slot(SECOND, second, self.second_curve.clone());
        // The shown child last, so that it is painted over the one leaving.
        let layers = if self.show_first {
            vec![second, first]
        } else {
            vec![first, second]
        };
        vec![Box::new(
            crate::AnimatedSize::new(self.duration, Cell { layers })
                .curve(self.size_curve.clone())
                .alignment(self.alignment),
        )]
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for AnimatedCrossFade<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn build_themed(&self, _theme: &Theme) {
        self.built.get_or_init(|| self.assemble());
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get().map(|v| &v[..]).unwrap_or(&[])
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "AnimatedCrossFade"
    }
}

/// Both children in one cell, stretched to it: the shown one gives the cell its size, and
/// the hidden one's box covers the cell without adding to it.
struct Cell<Msg> {
    layers: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for Cell<Msg> {
    fn style(&self) -> Style {
        Style {
            overlap: true,
            align: frus_layout::Align::Stretch,
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
        "AnimatedCrossFade"
    }
}

/// A child stretched across the width it is given, **asking for that width** so that the
/// wrappers around it — the fade, the taps, the semantics, none of which stretch what they
/// hold — are given it too (milestone 590's fill request, passed up through lone children).
struct FillWidth<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for FillWidth<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: frus_layout::FlexDirection::Column,
            align: frus_layout::Align::Stretch,
            ..Style::default()
        }
    }

    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: true,
            vertical: false,
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::Color;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        First,
        Second,
    }

    const SIZE: Size = Size::new(300.0, 600.0);
    const BLUE: Color = Color::rgb(0.2, 0.4, 0.8);
    const RED: Color = Color::rgb(0.8, 0.2, 0.2);

    /// A 40-px blue block that answers `First`, and a 100-px red one that answers `Second`,
    /// above a marker whose top says where the cross-fade ends.
    fn page(show_first: bool) -> crate::Flex<Msg> {
        // A coloured box **as tall as what it holds**, not one told its height: a box told
        // its height keeps it however it is laid out, and would hide a child squeezed to
        // the cross-fade's box.
        let block = |height: f32, color: Color, msg: Msg| {
            crate::GestureDetector::new(
                Container::<Msg>::new()
                    .color(color)
                    .child(Container::<Msg>::new().height(height)),
            )
            .on_tap(msg)
        };
        crate::Flex::<Msg>::column()
            .child(AnimatedCrossFade::new(
                block(40.0, BLUE, Msg::First),
                block(100.0, RED, Msg::Second),
                show_first,
                0.2,
            ))
            .child(
                Container::<Msg>::new()
                    .height(10.0)
                    .color(Color::rgb(0.1, 0.9, 0.1)),
            )
    }

    /// Where the marker starts: the bottom of the cross-fade.
    fn bottom(ui: &crate::Ui<Msg>) -> f32 {
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. }
                    if *color == Color::rgb(0.1, 0.9, 0.1) =>
                {
                    Some(rect.y)
                }
                _ => None,
            })
            .expect("the marker")
    }

    /// One frame, and the time after it: the runtime's own transitions and the tree's
    /// animated values — the fades — advanced as the shell advances them.
    fn frame(rt: &mut Runtime, show_first: bool, dt: f32) -> crate::Ui<Msg> {
        let tree = page(show_first);
        let theme = Theme::default();
        crate::build_deferred(&tree, &theme, rt);
        let ui = build_ui(&tree, SIZE, rt, &theme);
        rt.advance(dt);
        rt.advance_values(&tree, dt);
        ui
    }

    /// What a tap in the middle of the first 40 px answers.
    fn tap(ui: &crate::Ui<Msg>) -> Option<Msg> {
        ui.hit(frus_core::Point::new(150.0, 20.0))
            .and_then(|id| ui.msg_for(id))
    }

    /// The group opacity each colour is painted under, or `None` when it is not painted.
    fn painted(ui: &crate::Ui<Msg>, color: Color) -> Option<f32> {
        fn walk(primitives: &[frus_core::Primitive], color: Color, alpha: f32) -> Option<f32> {
            for p in primitives {
                match p {
                    frus_core::Primitive::Rect { color: c, .. } if *c == color => {
                        return Some(alpha)
                    }
                    frus_core::Primitive::Layer {
                        primitives,
                        opacity,
                        ..
                    } => {
                        if let Some(found) = walk(primitives, color, alpha * opacity) {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        walk(ui.scene().primitives(), color, 1.0).filter(|a| *a > 0.0)
    }

    /// **At rest, the box is the shown child's size**, only the shown child is painted, and
    /// only it takes a tap — the hidden one is taller and still adds nothing.
    #[test]
    fn at_rest_only_the_shown_child_counts() {
        let rt = Runtime::default();
        let ui = build_ui(&page(true), SIZE, &rt, &Theme::default());
        assert_eq!(bottom(&ui), 40.0);
        assert_eq!(painted(&ui, BLUE), Some(1.0));
        assert_eq!(painted(&ui, RED), None);
        assert_eq!(tap(&ui), Some(Msg::First));
    }

    /// **A switch fades one into the other and moves the box between their sizes**: half-way
    /// both are painted, the box is between 40 and 100 px and the marker below it follows,
    /// and at the end only the second is there, 100 px tall, taking the tap.
    #[test]
    fn a_switch_cross_fades_and_moves_the_box() {
        let mut rt = Runtime::default();
        frame(&mut rt, true, 0.016);
        frame(&mut rt, false, 0.1);
        let half = frame(&mut rt, false, 0.016);
        let b = bottom(&half);
        assert!(b > 45.0 && b < 95.0, "between the two sizes: {b}");
        let (blue, red) = (painted(&half, BLUE), painted(&half, RED));
        assert!(
            blue.is_some_and(|a| a < 1.0) && red.is_some_and(|a| a < 1.0),
            "both on the way: {blue:?} {red:?}"
        );
        assert_eq!(
            tap(&half),
            Some(Msg::Second),
            "the one leaving takes nothing"
        );
        // The one arriving is painted over the one leaving.
        let order: Vec<Color> = {
            fn walk(primitives: &[frus_core::Primitive], out: &mut Vec<Color>) {
                for p in primitives {
                    match p {
                        frus_core::Primitive::Rect { color, .. } => out.push(*color),
                        frus_core::Primitive::Layer { primitives, .. } => walk(primitives, out),
                        _ => {}
                    }
                }
            }
            let mut out = Vec::new();
            walk(half.scene().primitives(), &mut out);
            out
        };
        let blue_at = order.iter().position(|c| *c == BLUE);
        let red_at = order.iter().position(|c| *c == RED);
        assert!(red_at > blue_at, "the red one over the blue one: {order:?}");
        for _ in 0..20 {
            frame(&mut rt, false, 0.016);
        }
        let end = frame(&mut rt, false, 0.016);
        assert_eq!(bottom(&end), 100.0);
        assert_eq!(painted(&end, BLUE), None);
        assert_eq!(painted(&end, RED), Some(1.0));
    }

    /// **The hidden child takes no tap and is not read out**, even where the shown one has
    /// nothing of its own to answer with: a plain block is shown, a tappable, labelled one
    /// is hidden under it.
    #[test]
    fn the_hidden_child_is_neither_touched_nor_read() {
        let hidden = crate::Semantics::new(
            frus_core::SemanticsProperties::default(),
            // A width of its own: a `Semantics` stretches nothing it holds, and a block
            // with no width would be there to tap nowhere.
            crate::GestureDetector::new(
                Container::<Msg>::new()
                    .width(SIZE.width)
                    .height(100.0)
                    .color(RED),
            )
            .on_tap(Msg::Second),
        )
        .label("hidden");
        let tree = AnimatedCrossFade::new(
            Container::<Msg>::new().height(40.0).color(BLUE),
            hidden,
            true,
            0.2,
        );
        let rt = Runtime::default();
        let ui = build_ui(&tree, SIZE, &rt, &Theme::default());
        assert_eq!(tap(&ui), None, "nothing to tap on the shown child");
        assert!(
            !ui.semantics()
                .iter()
                .any(|(_, _, props)| props.label.as_deref() == Some("hidden")),
            "the hidden child is not read out"
        );
    }

    /// **The child on its way out keeps its own height and is cut to the box**: switching
    /// back from the 100-px red one, the red block is still 100 px tall as it fades, and
    /// nothing of it shows below the box.
    #[test]
    fn the_leaving_child_is_cut_to_the_box() {
        let mut rt = Runtime::default();
        frame(&mut rt, false, 0.016);
        frame(&mut rt, true, 0.1);
        let half = frame(&mut rt, true, 0.016);
        let edge = bottom(&half);
        fn find(primitives: &[frus_core::Primitive]) -> Option<(Rect, Rect)> {
            primitives.iter().find_map(|p| match p {
                frus_core::Primitive::Rect {
                    rect, clip, color, ..
                } if *color == RED => Some((*rect, *clip)),
                frus_core::Primitive::Layer { primitives, .. } => find(primitives),
                _ => None,
            })
        }
        let (rect, clip) = find(half.scene().primitives()).expect("the red block, fading");
        assert_eq!(rect.height, 100.0, "its own height");
        assert!(
            clip.y + clip.height <= edge + 0.5,
            "cut at {edge}: {clip:?}"
        );
        assert!(edge < 95.0, "the box is on its way down: {edge}");
    }
}
