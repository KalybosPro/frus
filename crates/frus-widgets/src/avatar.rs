//! [`Avatar`]: a round pill of initials, or of colour, in the accent colours — for
//! lists, headers and the like.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const SIZE: f32 = 36.0;

/// A circular avatar showing up to two initials.
pub struct Avatar {
    initials: String,
    color: Option<Color>,
    size: f32,
}

impl Avatar {
    /// Creates an avatar from a text: its first two letters, upper-cased.
    pub fn new(initials: impl Into<String>) -> Self {
        let source: String = initials.into();
        let initials: String = source
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();
        let initials = if initials.is_empty() {
            source.chars().take(2).collect::<String>().to_uppercase()
        } else {
            initials
        };
        Self {
            initials,
            color: None,
            size: SIZE,
        }
    }

    /// The background colour; the theme's accent otherwise.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The size, that is, the diameter, in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl<Msg> Widget<Msg> for Avatar {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.size),
            height: Dimension::Length(self.size),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let bg = self.color.unwrap_or(theme.primary);
        // A circle: a fully rounded rectangle.
        scene.draw_rect(
            bounds,
            bg.fade(o),
            bounds.height * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        let fsize = self.size * 0.4;
        let measured = frus_text::measure(&self.initials, fsize);
        scene.text(
            Point::new(
                bounds.x + (bounds.width - measured.width) * 0.5,
                bounds.y + (bounds.height - measured.height) * 0.5,
            ),
            self.initials.clone(),
            fsize,
            theme.on_primary.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[test]
    fn takes_two_uppercased_initials() {
        let a = Avatar::new("ada lovelace");
        assert_eq!(a.initials, "AL");
        let b = Avatar::new("bob");
        assert_eq!(b.initials, "B");
    }

    #[test]
    fn paints_circle_and_initials() {
        let a = Avatar::new("Zoe");
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &a,
            Rect::new(0.0, 0.0, 36.0, 36.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { .. })));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Z")));
    }
}
