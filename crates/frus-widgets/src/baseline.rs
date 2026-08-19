//! [`Baseline`] and [`IgnoreBaseline`]: the two widgets that say something about
//! **the line text sits on** rather than about a box.
//!
//! A baseline is the only measurement that makes two runs of different sizes read as
//! one line. Align two labels by their tops and the bigger one hangs; by their
//! bottoms and it hangs the other way; by their middles and neither sits anywhere a
//! reader recognises. By their baselines, they are simply on the same line.
//!
//! The alignment itself is not here — it is `Align::Baseline` on a row, which is
//! where a row's children are arranged. These two are the fine adjustments beside it:
//! one puts a baseline at a chosen height, the other takes a subtree out of the
//! reckoning.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Positions its child so that the **child's baseline** lands `baseline` logical
/// pixels below the top of this box, then grows to contain it.
///
/// It is the manual version of what `Align::Baseline` does automatically: a way to
/// pin one piece of text to a line decided elsewhere — a caption on a rule, a label
/// on a grid, a figure on a chart's axis.
///
/// If the child's own baseline is already lower than `baseline` there is nowhere to
/// push it up to, and the child is **top-aligned** instead. A child with no text in
/// it has no baseline; its bottom edge is used, which is what a plain box amounts to.
///
/// ```ignore
/// Baseline::new(24.0).child(Text::new("on the line"))
/// ```
///
/// There is no choice of baseline *kind*. The measurement here is the alphabetic
/// baseline — the line Latin, Greek and Cyrillic sit on — because that is the one the
/// text engine reports. An ideographic baseline is a different number and offering a
/// name for it that resolved to the same value would be worse than not offering it.
pub struct Baseline<Msg> {
    baseline: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Baseline<Msg> {
    /// Puts the child's baseline `baseline` pixels below the top of this box.
    pub fn new(baseline: f32) -> Self {
        Self {
            baseline,
            children: Vec::new(),
        }
    }

    /// Sets the positioned child.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Baseline<Msg> {
    fn style(&self) -> Style {
        // The shift is applied at layout time, as top padding, once the child's own
        // baseline has been measured — see `build_layout`.
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn baseline_target(&self) -> Option<f32> {
        Some(self.baseline)
    }
}

/// Takes its subtree **out of a parent's baseline alignment**: the text inside still
/// has a baseline, and the row above is to pretend it does not.
///
/// A row aligned on baselines lines up on the *lowest* one it can find, so a single
/// tall or oddly-placed piece of text drags everything else down with it. This is how
/// to say which text the line belongs to: an icon's label, a superscript, a badge —
/// present in the row, not consulted about where the line is.
///
/// ```ignore
/// Flex::row()
///     .align(Align::Baseline)
///     .child(Text::styled("12", theme.text.display_large))
///     .child(IgnoreBaseline::new().child(Text::styled("beta", theme.text.label_small)))
/// ```
///
/// It changes nothing else: the subtree lays out, paints and responds exactly as it
/// would have.
pub struct IgnoreBaseline<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> IgnoreBaseline<Msg> {
    /// Hides the subtree's baseline from whatever is above it.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Sets the subtree.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for IgnoreBaseline<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for IgnoreBaseline<Msg> {
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

    fn ignores_baseline(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Align, Container, Flex, Text};
    use frus_core::{Color, Primitive, Size, TextStyle};

    fn rects(root: &impl Widget<()>, size: Size) -> Vec<Rect> {
        let rt = crate::runtime::Runtime::default();
        let theme = Theme::dark();
        let ui = crate::ui::build_ui(root, size, &rt, &theme);
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { position, size, .. } => {
                    Some(Rect::new(position.x, position.y, 0.0, *size))
                }
                _ => None,
            })
            .collect()
    }

    /// The whole point, in one assertion: two labels of very different sizes in a
    /// baseline-aligned row sit on **one line**, which top alignment would not give.
    #[test]
    fn a_baseline_row_puts_two_sizes_on_one_line() {
        let row = |align: Align| {
            Flex::<()>::row()
                .align(align)
                .child(Text::styled("48", TextStyle::new(48.0)))
                .child(Text::styled("px", TextStyle::new(12.0)))
        };
        let baseline_of = |align: Align| {
            let boxes = rects(&row(align), Size::new(300.0, 120.0));
            assert_eq!(boxes.len(), 2, "two labels");
            // The painted position is the top of the line box; the baseline is that
            // plus the font's ascent, which is what the two are supposed to share.
            let line = |r: &Rect| {
                r.y + frus_text::baseline(r.height, frus_core::FontWeight::Regular, false)
            };
            (line(&boxes[0]), line(&boxes[1]))
        };
        let (big, small) = baseline_of(Align::Baseline);
        assert!((big - small).abs() < 1.0, "one line: {big} and {small}");
        // And without it they are not on one line at all, which is what makes the
        // assertion above worth making.
        let (big, small) = baseline_of(Align::Start);
        assert!(
            big - small > 10.0,
            "top-aligned, they are not: {big} {small}"
        );
    }

    /// `IgnoreBaseline` takes a subtree out of the reckoning: the row lines up on the
    /// label that is left, and the ignored one keeps the place top alignment gives it.
    #[test]
    fn an_ignored_subtree_does_not_move_the_line() {
        let row = |ignore: bool| {
            let tall = Text::styled("48", TextStyle::new(48.0));
            let mut flex = Flex::<()>::row()
                .align(Align::Baseline)
                .child(Text::styled("px", TextStyle::new(12.0)));
            flex = if ignore {
                flex.child(IgnoreBaseline::new().child(tall))
            } else {
                flex.child(tall)
            };
            flex
        };
        let small_y = |ignore: bool| rects(&row(ignore), Size::new(300.0, 120.0))[0].y;
        // Consulted, the tall label drags the small one down to meet it.
        assert!(small_y(false) > 1.0, "pushed down: {}", small_y(false));
        // Ignored, there is nothing left to line up with and the small label stays put.
        assert!(small_y(true) < 1.0, "left alone: {}", small_y(true));
    }

    /// `Baseline` puts the child's baseline at the height asked for.
    #[test]
    fn a_baseline_box_pins_the_line_it_was_given() {
        let root = Baseline::<()>::new(40.0).child(Text::styled("x", TextStyle::new(16.0)));
        let boxes = rects(&root, Size::new(200.0, 120.0));
        let line = boxes[0].y + frus_text::baseline(16.0, frus_core::FontWeight::Regular, false);
        assert!((line - 40.0).abs() < 1.0, "the line is at 40: {line}");
    }

    /// Asked for a line the child is already past, there is nowhere to push it up to
    /// and it is top-aligned instead — the answer the reference gives.
    #[test]
    fn a_baseline_above_the_child_leaves_it_at_the_top() {
        let root = Baseline::<()>::new(2.0).child(Text::styled("x", TextStyle::new(16.0)));
        let boxes = rects(&root, Size::new(200.0, 120.0));
        assert!(boxes[0].y < 0.5, "top-aligned: {}", boxes[0].y);
    }

    /// A child with no text has no baseline, and a row of such children is arranged
    /// exactly as a start-aligned row would be — no shifting, no surprises.
    #[test]
    fn boxes_without_text_are_not_moved() {
        let boxed = |align: Align| {
            let root = Flex::<()>::row()
                .align(align)
                .child(
                    Container::new()
                        .width(20.0)
                        .height(50.0)
                        .color(Color::WHITE),
                )
                .child(
                    Container::new()
                        .width(20.0)
                        .height(20.0)
                        .color(Color::WHITE),
                );
            let rt = crate::runtime::Runtime::default();
            let theme = Theme::dark();
            let ui = crate::ui::build_ui(&root, Size::new(200.0, 120.0), &rt, &theme);
            ui.scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, .. } => Some(rect.y),
                    _ => None,
                })
                .collect::<Vec<f32>>()
        };
        assert_eq!(boxed(Align::Baseline), boxed(Align::Start));
    }
}
