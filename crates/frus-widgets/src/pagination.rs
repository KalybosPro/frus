//! [`Pagination`]: a page picker — ‹ prev · a window of pages · next ›.

use frus_core::{Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// How many pages are shown on either side of the current one.
const WINDOW: usize = 2;

/// A page button: active, a link, or disabled.
fn page_button<Msg: Clone + 'static>(
    label: impl Into<String>,
    message: Option<Msg>,
    active: bool,
) -> Box<dyn Widget<Msg>> {
    let variant = if active {
        Variant::Primary
    } else {
        Variant::Secondary
    };
    let mut button = Button::new(label).variant(variant).size(15.0);
    if let Some(message) = message {
        button = button.on_press(message);
    }
    Box::new(button)
}

/// A controlled page picker; pages are **1-indexed**.
pub struct Pagination<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Pagination<Msg> {
    /// Creates the picker: page `current` of `total`, with `on_select(p)` on click.
    pub fn new(current: usize, total: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        let total = total.max(1);
        let current = current.clamp(1, total);
        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();

        // ‹ previous, disabled on the first page.
        children.push(page_button(
            "‹",
            (current > 1).then(|| on_select(current - 1)),
            false,
        ));

        // The window of pages around the current one.
        let start = current.saturating_sub(WINDOW).max(1);
        let end = (current + WINDOW).min(total);
        for page in start..=end {
            children.push(page_button(
                page.to_string(),
                Some(on_select(page)),
                page == current,
            ));
        }

        // › next, disabled on the last page.
        children.push(page_button(
            "›",
            (current < total).then(|| on_select(current + 1)),
            false,
        ));

        Self { children }
    }
}

impl<Msg: Clone> Widget<Msg> for Pagination<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 4.0,
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
        Page(usize),
    }

    #[test]
    fn windows_pages_and_bounds_prev_next() {
        // 10 pages, courante 5 → ‹ + [3 4 5 6 7] + › = 7 enfants.
        let p = Pagination::new(5, 10, Msg::Page);
        let children = Widget::<Msg>::children(&p);
        assert_eq!(children.len(), 7);
        // ‹ goes to page 4.
        assert_eq!(children[0].on_click(), Some(Msg::Page(4)));
        // › goes to page 6.
        assert_eq!(children[6].on_click(), Some(Msg::Page(6)));
    }

    #[test]
    fn first_page_disables_prev() {
        let p = Pagination::new(1, 3, Msg::Page);
        let children = Widget::<Msg>::children(&p);
        assert_eq!(children[0].on_click(), None); // ‹ disabled
    }
}
