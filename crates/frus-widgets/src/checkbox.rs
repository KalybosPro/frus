//! [`Checkbox`] : une case à cocher **contrôlée** (état venant de l'application).

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const BOX: f32 = 20.0;
const GAP: f32 = 10.0;

/// Une case à cocher, avec libellé optionnel.
pub struct Checkbox<Msg> {
    checked: bool,
    label: Option<String>,
    size: f32,
    on_toggle: Option<Box<dyn Fn(bool) -> Msg>>,
}

impl<Msg> Checkbox<Msg> {
    /// Crée une case dont l'état coché est fourni.
    pub fn new(checked: bool) -> Self {
        Self {
            checked,
            label: None,
            size: 18.0,
            on_toggle: None,
        }
    }

    /// Ajoute un libellé à droite.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Closure produisant un message depuis le nouvel état (coché ou non).
    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    fn label_width(&self) -> f32 {
        match &self.label {
            Some(text) => GAP + frus_text::measure(text, self.size).width,
            None => 0.0,
        }
    }
}

impl<Msg> Widget<Msg> for Checkbox<Msg> {
    fn style(&self) -> Style {
        let line = frus_text::line_height(self.size).max(BOX);
        Style {
            width: Dimension::Length((BOX + self.label_width()).ceil()),
            height: Dimension::Length(line.ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let box_y = bounds.y + (bounds.height - BOX) * 0.5;
        let box_rect = Rect::new(bounds.x, box_y, BOX, BOX);

        if self.checked {
            scene.draw_rect(
                box_rect,
                theme.primary.fade(o),
                5.0,
                0.0,
                Color::TRANSPARENT,
            );
            scene.text(
                Point::new(box_rect.x + 3.0, box_rect.y + 1.0),
                "✓".to_string(),
                self.size,
                theme.on_primary.fade(o),
            );
        } else {
            scene.draw_rect(
                box_rect,
                theme.surface.fade(o),
                5.0,
                2.0,
                theme.border.fade(o),
            );
        }

        if let Some(label) = &self.label {
            scene.text(
                Point::new(bounds.x + BOX + GAP, bounds.y),
                label.clone(),
                self.size,
                theme.on_surface.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_toggle.as_ref().map(|make| make(!self.checked))
    }

    fn focusable(&self) -> bool {
        true
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        let mut s = frus_core::Semantics::new(frus_core::Role::CheckBox)
            .toggled(self.checked)
            .clickable();
        if let Some(label) = &self.label {
            s = s.label(label.clone());
        }
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Set(bool),
    }

    #[test]
    fn click_toggles() {
        let unchecked = Checkbox::new(false).on_toggle(Msg::Set);
        assert_eq!(Widget::on_click(&unchecked), Some(Msg::Set(true)));
        let checked = Checkbox::new(true).on_toggle(Msg::Set);
        assert_eq!(Widget::on_click(&checked), Some(Msg::Set(false)));
    }
}
