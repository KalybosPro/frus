//! [`NavScaffold`]: the **adaptive** navigation scaffold. Depending on the
//! [`SizeClass`] it places the primary navigation in a **vertical rail**
//! (Medium/Expanded) or in a **bottom bar** (Compact), the body filling the rest. This
//! is where the size class drives the screen's *structure*.

use frus_core::{Rect, Scene, SizeClass};
use frus_layout::{FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::navrail::{BottomBar, NavigationRail};
use crate::theme::Theme;
use crate::widget::Widget;

/// Adaptive navigation shell: a rail ↔ a bottom bar, depending on the size.
pub struct NavScaffold<Msg> {
    compact: bool,
    selected: usize,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    destinations: Vec<(String, String, Option<u32>)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavScaffold<Msg> {
    /// Creates a scaffold: `class` decides the presentation, `selected` the active
    /// destination, and `on_select(i)` is emitted when a destination is chosen.
    pub fn new(
        class: SizeClass,
        selected: usize,
        on_select: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        Self {
            compact: class == SizeClass::Compact,
            selected,
            on_select: Some(Box::new(on_select)),
            destinations: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Adds a destination, a glyph plus a label. Call this **before** [`body`].
    ///
    /// [`body`]: NavScaffold::body
    pub fn destination(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.destinations.push((icon.into(), label.into(), None));
        self
    }

    /// Adds a notification count to the **last** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.destinations.last_mut() {
            last.2 = Some(count);
        }
        self
    }

    /// Sets the body and **finalises** the scaffold; call it last. The body fills the
    /// space left beside, or above, the navigation.
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        let on_select = self.on_select.take().expect("body called exactly once");
        let destinations = std::mem::take(&mut self.destinations);
        let body_pane: Box<dyn Widget<Msg>> = Box::new(Flex::column().flex(1.0).child(body));

        // Only one arm runs, so `on_select` is moved exactly once.
        let nav: Box<dyn Widget<Msg>> = if self.compact {
            let mut bar = BottomBar::new(self.selected, on_select);
            for (icon, label, badge) in destinations {
                bar = bar.item(icon, label);
                if let Some(count) = badge {
                    bar = bar.badge(count);
                }
            }
            Box::new(bar)
        } else {
            let mut rail = NavigationRail::new(self.selected, on_select);
            for (icon, label, badge) in destinations {
                rail = rail.item(icon, label);
                if let Some(count) = badge {
                    rail = rail.badge(count);
                }
            }
            Box::new(rail)
        };

        // Compact puts the body above and the bar below; otherwise the rail is on the
        // left and the body on the right.
        self.children = if self.compact {
            vec![body_pane, nav]
        } else {
            vec![nav, body_pane]
        };
        self
    }
}

impl<Msg: Clone> Widget<Msg> for NavScaffold<Msg> {
    /// It asks to fill **both** axes rather than declaring `100%` on either — see
    /// [`FillAxes`](crate::widget::FillAxes). A percentage resolves against the parent's
    /// *resolved* size, which a parent that shrink-wraps has not got yet, so a shell
    /// nested in one vanished entirely (milestone 404).
    fn style(&self) -> Style {
        Style {
            flex_direction: if self.compact {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            ..Default::default()
        }
    }

    /// A shell takes everything it is offered, on both axes.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::BOTH
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
    }

    fn scaffold(class: SizeClass) -> NavScaffold<Msg> {
        NavScaffold::new(class, 0, Msg::Go)
            .destination("H", "Home")
            .destination("S", "Settings")
            .body(Container::new())
    }

    #[test]
    fn compact_puts_body_first_then_bottom_bar_in_a_column() {
        let s = scaffold(SizeClass::Compact);
        assert_eq!(
            Widget::<Msg>::style(&s).flex_direction,
            FlexDirection::Column
        );
        // [body, bar]: navigation is the last child, at the bottom.
        assert_eq!(Widget::<Msg>::children(&s).len(), 2);
    }

    #[test]
    fn expanded_puts_rail_first_then_body_in_a_row() {
        let s = scaffold(SizeClass::Expanded);
        assert_eq!(Widget::<Msg>::style(&s).flex_direction, FlexDirection::Row);
        // The rail (1st child) has a fixed width; the body takes the rest.
        let children = Widget::<Msg>::children(&s);
        assert_eq!(
            Widget::<Msg>::style(&*children[0]).width,
            frus_layout::Dimension::Length(crate::navrail::RAIL_WIDTH)
        );
    }
}
