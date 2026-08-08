//! [`Chip`]: a small label — a tag or a filter — optionally **removable**.

use frus_core::{Color, Insets, Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

const REMOVE: f32 = 16.0;

/// A chip's (clickable) delete cross.
struct Remove<Msg> {
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Remove<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(REMOVE),
            height: Dimension::Length(REMOVE),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let color = theme.muted.lerp(theme.on_surface, status.hover_progress);
        scene.text(
            Point::new(bounds.x + 3.0, bounds.y - 1.0),
            "×".to_string(),
            SIZE,
            color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

const SIZE: f32 = 15.0;

/// A compact label, in the theme's colours.
pub struct Chip<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Chip<Msg> {
    /// Creates a chip with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            children: vec![Box::new(Text::new(label).size(SIZE))],
        }
    }

    /// Adds a remove cross that emits `message`.
    pub fn on_remove(mut self, message: Msg) -> Self {
        self.children.truncate(1);
        self.children.push(Box::new(Remove { message }));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Chip<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            gap: 4.0,
            padding: Insets::new(3.0, 10.0, 3.0, 12.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Background pill, under the content.
        scene.draw_rect(
            bounds,
            theme.muted.fade(0.2 * o),
            bounds.height * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Point as P, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Remove,
    }

    #[test]
    fn plain_chip_has_label_only() {
        let chip: Chip<Msg> = Chip::new("tag");
        assert_eq!(Widget::<Msg>::children(&chip).len(), 1);
    }

    #[test]
    fn removable_chip_emits_message_on_cross() {
        let chip = Chip::new("filtre").on_remove(Msg::Remove);
        let ui = build_ui(
            &chip,
            Size::new(200.0, 40.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The cross is painted…
        assert!(ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "×")));
        // …and a point in its area, found by sweeping, emits the removal.
        let found = (0..40)
            .flat_map(|y| (0..200).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                ui.hit(P::new(x as f32, y as f32))
                    .and_then(|id| ui.msg_for(id))
            })
            .next();
        assert_eq!(found, Some(Msg::Remove));
    }
}
