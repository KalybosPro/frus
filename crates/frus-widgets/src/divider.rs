//! [`Divider`] : un fin séparateur horizontal, aux couleurs du thème.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un séparateur : une ligne fine qui occupe toute la largeur disponible.
pub struct Divider;

impl Divider {
    /// Crée un séparateur.
    pub fn new() -> Self {
        Divider
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Divider {
    fn style(&self) -> Style {
        Style {
            // Largeur automatique : étirée par le parent (align: stretch).
            width: Dimension::Auto,
            height: Dimension::Length(1.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        scene.fill_rect(bounds, theme.border.fade(status.opacity));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Primitive};

    #[test]
    fn paints_a_border_line() {
        let divider = Divider::new();
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &divider,
            Rect::new(0.0, 0.0, 120.0, 1.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert_eq!(scene.primitives().len(), 1);
        let border: Color = Theme::default().border;
        assert!(matches!(
            scene.primitives()[0],
            Primitive::Rect { color, .. } if color == border
        ));
    }
}
