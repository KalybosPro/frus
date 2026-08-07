//! [`TwoPane`]: the responsive **master-detail** pattern. In `Expanded`, the list
//! and the detail sit **side by side**; otherwise a **single pane** is shown (the
//! list, or the detail if `show_detail` — the app decides as it navigates).

use frus_core::{Rect, Scene, SizeClass};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// An adaptive master-detail layout.
pub struct TwoPane<Msg> {
    class: SizeClass,
    ratio: f32,
    show_detail: bool,
    list: Option<Box<dyn Widget<Msg>>>,
    row: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> TwoPane<Msg> {
    /// Creates a layout for size class `class` (the list ratio defaults to 0.38).
    pub fn new(class: SizeClass) -> Self {
        Self {
            class,
            ratio: 0.38,
            show_detail: false,
            list: None,
            row: false,
            children: Vec::new(),
        }
    }

    /// Fraction of the width given to the list in side-by-side mode (`0.1..=0.9`).
    /// Set this **before** [`detail`](TwoPane::detail).
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.1, 0.9);
        self
    }

    /// In single-pane mode, shows the **detail** rather than the list (the app sets
    /// this to `true` when it navigates to an item).
    pub fn show_detail(mut self, show: bool) -> Self {
        self.show_detail = show;
        self
    }

    /// Sets the **list** (the master pane). Call before [`detail`](TwoPane::detail).
    pub fn list(mut self, list: impl Widget<Msg> + 'static) -> Self {
        self.list = Some(Box::new(list));
        self
    }

    /// Sets the **detail** and **finalises** the layout (call this last).
    pub fn detail(mut self, detail: impl Widget<Msg> + 'static) -> Self {
        let detail: Box<dyn Widget<Msg>> = Box::new(detail);
        if self.class == SizeClass::Expanded {
            // Side by side: proportional widths via flex_grow.
            let list = self
                .list
                .take()
                .expect("list() before detail() in side-by-side mode");
            self.children = vec![
                Box::new(Flex::column().flex(self.ratio).child(list)),
                Box::new(Flex::column().flex(1.0 - self.ratio).child(detail)),
            ];
            self.row = true;
        } else {
            // A single pane: the detail if asked for, otherwise the list.
            let single = if self.show_detail {
                detail
            } else {
                self.list
                    .take()
                    .expect("single-pane mode needs list() or show_detail")
            };
            self.children = vec![Box::new(Flex::column().flex(1.0).child(single))];
            self.row = false;
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for TwoPane<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            flex_direction: if self.row {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
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
    use crate::Container;

    #[test]
    fn expanded_shows_both_panes_side_by_side() {
        let tp = TwoPane::<()>::new(SizeClass::Expanded)
            .ratio(0.4)
            .list(Container::new())
            .detail(Container::new());
        assert_eq!(Widget::<()>::style(&tp).flex_direction, FlexDirection::Row);
        assert_eq!(Widget::<()>::children(&tp).len(), 2);
        // List pane = flex 0.4, detail pane = flex 0.6.
        let panes = Widget::<()>::children(&tp);
        assert_eq!(Widget::<()>::style(&*panes[0]).flex_grow, 0.4);
        assert!((Widget::<()>::style(&*panes[1]).flex_grow - 0.6).abs() < 1e-6);
    }

    #[test]
    fn compact_shows_a_single_pane() {
        let list_only = TwoPane::<()>::new(SizeClass::Compact)
            .list(Container::new())
            .detail(Container::new());
        assert_eq!(
            Widget::<()>::style(&list_only).flex_direction,
            FlexDirection::Column
        );
        assert_eq!(Widget::<()>::children(&list_only).len(), 1);

        // With show_detail, it is the detail that occupies the single pane.
        let detail_shown = TwoPane::<()>::new(SizeClass::Compact)
            .show_detail(true)
            .list(Container::new())
            .detail(Container::new());
        assert_eq!(Widget::<()>::children(&detail_shown).len(), 1);
    }
}
