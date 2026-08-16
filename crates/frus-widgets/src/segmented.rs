//! [`SegmentedControl`]: a **controlled** segmented picker — connected buttons with a
//! single selection, in the manner of an iOS segmented control.

use frus_core::{BorderRadius, Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A single-selection segmented control.
pub struct SegmentedControl<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Vec<String>,
    /// The radius of the **outer** corners; the inner ones stay square, since the
    /// segments are connected. It defaults to 10 px and `radius` overrides it.
    radius: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SegmentedControl<Msg> {
    /// Creates a control: `selected` is the active index, `on_select(i)` fires on click.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            labels: Vec::new(),
            radius: 10.0,
            children: Vec::new(),
        }
    }

    /// Overrides the radius of the group's outer corners.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self.rebuild();
        self
    }

    /// Ajoute un segment.
    pub fn segment(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    /// The radii of segment `i` of `n`: only the group's **outer** corners are
    /// rounded — the first on the left, the last on the right — while the joints stay
    /// square, which gives the connected-buttons look.
    fn corner_radius(&self, i: usize, n: usize) -> BorderRadius {
        let r = self.radius;
        match (i == 0, i + 1 == n) {
            (true, true) => BorderRadius::uniform(r),
            (true, false) => BorderRadius {
                top_left: r,
                bottom_left: r,
                top_right: 0.0,
                bottom_right: 0.0,
            },
            (false, true) => BorderRadius {
                top_right: r,
                bottom_right: r,
                top_left: 0.0,
                bottom_left: 0.0,
            },
            (false, false) => BorderRadius::ZERO,
        }
    }

    fn rebuild(&mut self) {
        let count = self.labels.len();
        self.children = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let variant = if i == self.selected {
                    Variant::Filled
                } else {
                    Variant::Outlined
                };
                Box::new(
                    Button::new(label.clone())
                        .variant(variant)
                        .size(15.0)
                        .radius(self.corner_radius(i, count))
                        .on_press((self.on_select)(i)),
                ) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for SegmentedControl<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 2.0,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Select(usize),
    }

    #[test]
    fn segments_emit_index_and_highlight_selected() {
        let seg = SegmentedControl::new(1, Msg::Select)
            .segment("Day")
            .segment("Week")
            .segment("Month");
        let children = Widget::<Msg>::children(&seg);
        assert_eq!(children.len(), 3);
        // Clicking the 3rd segment → Select(2).
        assert_eq!(children[2].on_click(), Some(Msg::Select(2)));
    }

    #[test]
    fn segments_round_only_the_outer_corners() {
        use crate::{build_ui, Runtime, Size, Theme};
        let seg = SegmentedControl::new(0, Msg::Select)
            .segment("One")
            .segment("Two")
            .segment("Three");
        let ui = build_ui(
            &seg,
            Size::new(400.0, 60.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The buttons' boxes — unblurred rectangles — in order: first, middle, last.
        // Not filtered by opacity: an unselected segment is an outline over nothing,
        // which is what an outlined button became in milestone 313.
        let fills: Vec<BorderRadius> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect {
                    radius,
                    blur,
                    color,
                    ..
                } if *blur == 0.0 => {
                    let _ = color;
                    Some(*radius)
                }
                _ => None,
            })
            .collect();
        assert_eq!(fills.len(), 3, "three segment boxes");
        assert!(
            fills[0].top_left > 0.0 && fills[0].top_right == 0.0,
            "1st: left rounded"
        );
        assert_eq!(fills[1], BorderRadius::ZERO, "middle: square");
        assert!(
            fills[2].top_right > 0.0 && fills[2].top_left == 0.0,
            "last: right rounded"
        );
    }
}
