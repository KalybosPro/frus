//! [`Card`]: a themed surface — background, border, radius, shadow — with one child.

use frus_core::{Insets, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A card: a surface container, in the theme's colours.
pub struct Card<Msg> {
    padding: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Card<Msg> {
    /// Creates a card, with a default padding of 16.
    pub fn new() -> Self {
        Self {
            padding: 16.0,
            children: Vec::new(),
        }
    }

    /// Uniform padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the card's content.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for Card<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Card<Msg> {
    fn style(&self) -> Style {
        Style {
            padding: Insets::uniform(self.padding),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let blur = 12.0;
        scene.shadow(
            Rect::new(
                bounds.x - blur,
                bounds.y + 4.0 - blur,
                bounds.width + 2.0 * blur,
                bounds.height + 2.0 * blur,
            ),
            theme.scheme.shadow.with_alpha(0.30).fade(o),
            theme.radius + blur,
            blur,
        );
        scene.draw_rect(
            bounds,
            theme.surface.fade(o),
            theme.radius,
            1.0,
            theme.border.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}
