//! [`Text`] : un widget affichant une ligne de texte.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un widget de texte sur une ligne.
///
/// Sa taille de mise en page est calculée par mesure (`frus-text`) ; il pousse
/// une primitive de texte dans la scène lors de la peinture.
pub struct Text {
    content: String,
    size: f32,
    /// Couleur explicite ; sinon `theme.on_surface`.
    color: Option<Color>,
}

impl Text {
    /// Crée un texte (taille 16 px, couleur du thème par défaut).
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 16.0,
            color: None,
        }
    }

    /// Fixe la taille de police, en pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Fixe la couleur du texte (sinon celle du thème).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl<Msg> Widget<Msg> for Text {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.content, self.size);
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(measured.height.ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self.color.unwrap_or(theme.on_surface).fade(status.opacity);
        scene.text(Point::new(bounds.x, bounds.y), self.content.clone(), self.size, color);
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
    fn text_paints_a_text_primitive() {
        let text = Text::new("Salut").size(20.0).color(Color::rgb(1.0, 0.0, 0.0));
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &text,
            Rect::new(5.0, 6.0, 100.0, 24.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );

        assert_eq!(scene.primitives().len(), 1);
        assert_eq!(
            scene.primitives()[0],
            Primitive::Text {
                position: Point::new(5.0, 6.0),
                text: "Salut".to_string(),
                size: 20.0,
                color: Color::rgb(1.0, 0.0, 0.0),
                clip: Rect::UNBOUNDED,
                owner: 0,
            }
        );
    }
}
