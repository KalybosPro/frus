//! [`Popover`]: a floating panel with **free content**, anchored and controlled, that
//! closes on an outside click. It generalises [`crate::Menu`] to any content.

use frus_core::{Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// A popover: an anchor plus an optional floating panel.
pub struct Popover<Msg> {
    open: bool,
    /// `[ancre]` ou `[ancre, contenu]`.
    children: Vec<Box<dyn Widget<Msg>>>,
    on_dismiss: Option<Msg>,
}

impl<Msg: Clone + 'static> Popover<Msg> {
    /// Creates a popover around an anchor. When `open`, the content floats;
    /// `on_dismiss` is emitted on a click **outside** the popover.
    pub fn new(anchor: impl Widget<Msg> + 'static, open: bool, on_dismiss: Msg) -> Self {
        Self {
            open,
            children: vec![Box::new(anchor)],
            on_dismiss: if open { Some(on_dismiss) } else { None },
        }
    }

    /// Sets the floating content, which is shown only when open.
    pub fn content(mut self, content: impl Widget<Msg> + 'static) -> Self {
        if self.open {
            self.children.truncate(1);
            self.children.push(Box::new(content));
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Popover<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
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

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children
            .get(1)
            .map(|content| (content.as_ref(), Placement::Below))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Text};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
    }

    fn anchor() -> Container<Msg> {
        Container::<Msg>::new().width(40.0).height(30.0)
    }

    #[test]
    fn closed_has_no_overlay() {
        let p = Popover::new(anchor(), false, Msg::Close).content(Text::new("hi"));
        assert!(Widget::<Msg>::overlay(&p).is_none());
    }

    #[test]
    fn open_floats_content_and_dismisses() {
        let p = Popover::new(anchor(), true, Msg::Close).content(Text::new("hi"));
        assert!(Widget::<Msg>::overlay(&p).is_some());
        assert_eq!(Widget::<Msg>::overlay_dismiss(&p), Some(Msg::Close));
    }
}
