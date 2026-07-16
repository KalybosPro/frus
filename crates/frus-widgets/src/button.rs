//! [`Button`] : un bouton themé avec libellé, variantes et états d'interaction.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 20.0;
const PAD_Y: f32 = 12.0;

/// Variante visuelle d'un bouton.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Variant {
    /// Accent principal du thème.
    #[default]
    Primary,
    /// Surface neutre bordée.
    Secondary,
    /// Action destructive.
    Danger,
}

/// Un bouton cliquable.
pub struct Button<Msg> {
    label: String,
    size: f32,
    variant: Variant,
    on_press: Option<Msg>,
}

impl<Msg> Button<Msg> {
    /// Crée un bouton avec un libellé.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 18.0,
            variant: Variant::Primary,
            on_press: None,
        }
    }

    /// Choisit la variante visuelle.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Taille de police.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Message émis au clic.
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// (base, texte, bordure) selon la variante et le thème.
    fn palette(&self, theme: &Theme) -> (Color, Color, Option<Color>) {
        match self.variant {
            Variant::Primary => (theme.primary, theme.on_primary, None),
            Variant::Secondary => (theme.surface, theme.on_surface, Some(theme.border)),
            Variant::Danger => (theme.error, theme.on_error, None),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Button<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.label, self.size);
        Style {
            width: Dimension::Length((measured.width + PAD_X * 2.0).ceil()),
            height: Dimension::Length((measured.height + PAD_Y * 2.0).ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let (base, on_color, border) = self.palette(theme);

        // État survol/pression/focus via la state-layer bakée du thème.
        let color = theme.state_layer(base, on_color, &status);

        let blur = 10.0;
        let shadow_rect = Rect::new(
            bounds.x - blur,
            bounds.y + 3.0 - blur,
            bounds.width + 2.0 * blur,
            bounds.height + 2.0 * blur,
        );
        scene.shadow(
            shadow_rect,
            Color::rgba(0.0, 0.0, 0.0, 0.35).fade(o),
            theme.radius + blur,
            blur,
        );
        let (bw, bc) = match border {
            Some(c) => (1.0, c.fade(o)),
            None => (0.0, Color::TRANSPARENT),
        };
        scene.draw_rect(bounds, color.fade(o), theme.radius, bw, bc);
        scene.text(
            Point::new(bounds.x + PAD_X, bounds.y + PAD_Y),
            self.label.clone(),
            self.size,
            on_color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_press.clone()
    }

    fn focusable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pressed,
    }

    #[test]
    fn on_click_returns_message() {
        let button = Button::new("OK").on_press(Msg::Pressed);
        assert_eq!(Widget::on_click(&button), Some(Msg::Pressed));
    }
}
