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
///
/// **A button with no message is disabled**, not merely silent. Until milestone 324 the
/// arrows at the ends of the range were built without an `on_press` and left otherwise
/// untouched, so they were painted as live outlined buttons and Tab stopped on them: a
/// control that looks pressable, is pressable, and does nothing. The comments here already
/// said "disabled"; only the code did not.
fn page_button<Msg: Clone + 'static>(
    label: impl Into<String>,
    message: Option<Msg>,
    active: bool,
    enabled: bool,
) -> Box<dyn Widget<Msg>> {
    let variant = if active {
        Variant::Filled
    } else {
        Variant::Outlined
    };
    let enabled = enabled && message.is_some();
    // A page number, or an arrow: one or two characters. A button sized for a word would
    // make the strip several times as wide as the numbers in it — the reference would use
    // an icon button here, and there is none yet (milestone 313).
    let mut button = Button::new(label)
        .variant(variant)
        .size(15.0)
        .min_width(crate::button::BUTTON_HEIGHT)
        .padding(8.0)
        .enabled(enabled);
    if let Some(message) = message {
        button = button.on_press(message);
    }
    Box::new(button)
}

/// A controlled page picker; pages are **1-indexed**.
pub struct Pagination<Msg> {
    current: usize,
    total: usize,
    enabled: bool,
    on_select: Box<dyn Fn(usize) -> Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Pagination<Msg> {
    /// Creates the picker: page `current` of `total`, with `on_select(p)` on click.
    pub fn new(current: usize, total: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        let total = total.max(1);
        let mut pagination = Self {
            current: current.clamp(1, total),
            total,
            enabled: true,
            on_select: Box::new(on_select),
            children: Vec::new(),
        };
        pagination.rebuild();
        pagination
    }

    /// Whether pages can be picked. Disabled the whole strip is **inert** - no arrow, no
    /// number - and it still shows which page you are on.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// (Re)builds the strip from the current page and the total.
    fn rebuild(&mut self) {
        let (current, total) = (self.current, self.total);
        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();

        // ‹ previous, disabled on the first page.
        children.push(page_button(
            "‹",
            (current > 1).then(|| (self.on_select)(current - 1)),
            false,
            self.enabled,
        ));

        // The window of pages around the current one.
        let start = current.saturating_sub(WINDOW).max(1);
        let end = (current + WINDOW).min(total);
        for page in start..=end {
            children.push(page_button(
                page.to_string(),
                Some((self.on_select)(page)),
                page == current,
                self.enabled,
            ));
        }

        // › next, disabled on the last page.
        children.push(page_button(
            "›",
            (current < total).then(|| (self.on_select)(current + 1)),
            false,
            self.enabled,
        ));

        self.children = children;
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

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // Which page you are on is the whole of what the strip says, and it is still owed
        // to a reader who cannot change it.
        let semantics = frus_core::Semantics::new(frus_core::Role::None)
            .label(format!("Page {} of {}", self.current, self.total));
        Some(if self.enabled {
            semantics
        } else {
            semantics.disabled(true)
        })
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
        // 10 pages, current 5 → ‹ + [3 4 5 6 7] + › = 7 children.
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

    /// The arrows at the ends of the range said "disabled" in a comment and were painted
    /// live: no message, but a full outline, and Tab stopped on them.
    #[test]
    fn an_arrow_at_the_end_of_the_range_is_disabled_not_merely_silent() {
        let first = Pagination::new(1, 5, Msg::Page);
        let kids = Widget::<Msg>::children(&first);
        assert_eq!(kids[0].on_click(), None, "the back arrow does nothing");
        assert!(!kids[0].focusable(), "and Tab does not stop on it");
        assert!(
            kids.last().unwrap().focusable(),
            "while the forward arrow is live"
        );

        let last = Pagination::new(5, 5, Msg::Page);
        let kids = Widget::<Msg>::children(&last);
        assert!(!kids.last().unwrap().focusable(), "and the other way round");
        assert!(kids[0].focusable());
    }

    #[test]
    fn a_disabled_strip_is_inert_but_still_says_which_page() {
        let dead = Pagination::new(3, 5, Msg::Page).enabled(false);
        for (i, child) in Widget::<Msg>::children(&dead).iter().enumerate() {
            assert_eq!(child.on_click(), None, "child {i} still answers");
            assert!(!child.focusable(), "child {i} is still in the tab order");
        }
        let semantics = Widget::<Msg>::semantics(&dead).expect("still announced");
        assert!(semantics.disabled);
        assert_eq!(semantics.label.as_deref(), Some("Page 3 of 5"));
    }
}
