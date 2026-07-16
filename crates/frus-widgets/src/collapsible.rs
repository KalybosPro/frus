//! [`Collapsible`] : une section **repliable** contrôlée (en-tête cliquable +
//! contenu affiché si ouvert). L'apparition/disparition du contenu profite des
//! fondus de montage/démontage.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const HEADER_H: f32 = 40.0;
const PAD_X: f32 = 12.0;
const SIZE: f32 = 18.0;

/// En-tête cliquable : titre + chevron, bascule l'ouverture.
struct Header<Msg> {
    title: String,
    open: bool,
    on_toggle: Msg,
}

impl<Msg: Clone> Widget<Msg> for Header<Msg> {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(HEADER_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let bg = theme.state_layer(theme.surface, theme.on_surface, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.border.fade(o));
        let ty = bounds.y + (HEADER_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(
            Point::new(bounds.x + PAD_X, ty),
            if self.open { "▾" } else { "▸" }.to_string(),
            SIZE,
            theme.muted.fade(o),
        );
        scene.text(
            Point::new(bounds.x + PAD_X + 22.0, ty),
            self.title.clone(),
            SIZE,
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.on_toggle.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Une section repliable.
pub struct Collapsible<Msg> {
    open: bool,
    /// `[en-tête]` ou `[en-tête, contenu]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Collapsible<Msg> {
    /// Crée une section : titre, état ouvert, message de bascule.
    pub fn new(title: impl Into<String>, open: bool, on_toggle: Msg) -> Self {
        let header = Header {
            title: title.into(),
            open,
            on_toggle,
        };
        Self {
            open,
            children: vec![Box::new(header)],
        }
    }

    /// Définit le contenu (affiché seulement si ouvert).
    pub fn content(mut self, content: impl Widget<Msg> + 'static) -> Self {
        if self.open {
            self.children.truncate(1);
            self.children.push(Box::new(content));
        }
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Collapsible<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            gap: 8.0,
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
        Toggle,
    }

    #[test]
    fn collapsed_shows_header_only() {
        let section = Collapsible::new("Titre", false, Msg::Toggle).content(Text::new("caché"));
        assert_eq!(Widget::<Msg>::children(&section).len(), 1);
    }

    #[test]
    fn expanded_shows_content_and_header_toggles() {
        let section = Collapsible::new("Titre", true, Msg::Toggle).content(Text::new("visible"));
        let children = Widget::<Msg>::children(&section);
        assert_eq!(children.len(), 2);
        // L'en-tête émet la bascule.
        assert_eq!(children[0].on_click(), Some(Msg::Toggle));
    }
}
