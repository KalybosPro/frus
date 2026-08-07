//! [`Tabs`]: a **controlled** tab bar plus the selected panel.
//!
//! It is composite: its children are `[header, panel]`, in a column. Only the selected
//! tab's content is realised, the app rebuilding the view every frame.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::flex::Flex;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A tabbed view.
pub struct Tabs<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Vec<String>,
    /// Either `[header]` or `[header, panel]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Tabs<Msg> {
    /// Creates tabs: `selected` is the active index, `on_select(i)` the message on click.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        let mut tabs = Self {
            selected,
            on_select: Box::new(on_select),
            labels: Vec::new(),
            children: Vec::new(),
        };
        tabs.rebuild_header();
        tabs
    }

    /// Adds a tab, a label plus content. The content is realised only when it belongs
    /// to the selected tab.
    pub fn tab(mut self, label: impl Into<String>, content: impl Widget<Msg> + 'static) -> Self {
        let index = self.labels.len();
        self.labels.push(label.into());
        self.rebuild_header();
        if index == self.selected {
            if self.children.len() > 1 {
                self.children[1] = Box::new(content);
            } else {
                self.children.push(Box::new(content));
            }
        }
        self
    }

    /// (Re)builds the header, the tab buttons, at index 0.
    fn rebuild_header(&mut self) {
        let mut header = Flex::row().gap(6.0);
        for (i, label) in self.labels.iter().enumerate() {
            let variant = if i == self.selected {
                Variant::Primary
            } else {
                Variant::Secondary
            };
            header = header.child(
                Button::new(label.clone())
                    .variant(variant)
                    .size(15.0)
                    .on_press((self.on_select)(i)),
            );
        }
        let header: Box<dyn Widget<Msg>> = Box::new(header);
        if self.children.is_empty() {
            self.children.push(header);
        } else {
            self.children[0] = header;
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Tabs<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            gap: 12.0,
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
    use crate::Text;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Select(usize),
    }

    #[test]
    fn shows_header_and_selected_panel() {
        let tabs = Tabs::new(1, Msg::Select)
            .tab("One", Text::new("panel one"))
            .tab("Two", Text::new("panel two"))
            .tab("Three", Text::new("panel three"));
        // [header, panel] — the panel is the selected tab's content, tab 1.
        let children = Widget::<Msg>::children(&tabs);
        assert_eq!(children.len(), 2);
        // The header has 3 buttons.
        assert_eq!(children[0].children().len(), 3);
    }

    #[test]
    fn no_panel_when_selection_out_of_range() {
        let tabs = Tabs::new(9, Msg::Select).tab("One", Text::new("x"));
        assert_eq!(Widget::<Msg>::children(&tabs).len(), 1); // the header alone
    }
}
