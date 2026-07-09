//! [`TextInput`] : un champ de saisie mono-ligne, **contrôlé** (sa valeur vient
//! de l'état applicatif) et focalisable au clavier.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::{Key, Status};
use crate::widget::Widget;

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 6.0;

const BG: Color = Color::rgb(0.16, 0.17, 0.20);
const TEXT_COLOR: Color = Color::rgb(0.92, 0.92, 0.95);
const BORDER_IDLE: Color = Color::rgb(0.32, 0.34, 0.40);
const BORDER_FOCUS: Color = Color::rgb(0.35, 0.62, 0.95);

/// Un champ de saisie de texte sur une ligne.
///
/// Contrôlé : la valeur affichée est celle passée à [`TextInput::new`] (issue de
/// l'état applicatif). À la saisie, il émet un message construit par la closure
/// fournie à [`TextInput::on_input`] ; l'application met à jour son état, et la
/// frame suivante reflète la nouvelle valeur.
pub struct TextInput<Msg> {
    value: String,
    size: f32,
    width: Dimension,
    on_input: Option<Box<dyn Fn(String) -> Msg>>,
}

impl<Msg> TextInput<Msg> {
    /// Crée un champ affichant `value`.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            size: 18.0,
            width: Dimension::Length(220.0),
            on_input: None,
        }
    }

    /// Fixe la largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la taille de police, en pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Closure produisant un message depuis la nouvelle valeur du champ.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Calcule la nouvelle valeur après une touche (append / backspace).
    fn apply_key(&self, key: &Key) -> Option<String> {
        let mut value = self.value.clone();
        match key {
            Key::Text(text) => value.push_str(text),
            Key::Backspace => {
                value.pop();
            }
            // Pas de navigation / soumission pour l'instant.
            Key::Enter => return None,
        }
        Some(value)
    }
}

impl<Msg> Widget<Msg> for TextInput<Msg> {
    fn style(&self) -> Style {
        let height = frus_text::line_height(self.size) + PAD_Y * 2.0;
        Style {
            width: self.width,
            height: Dimension::Length(height.ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, scene: &mut Scene) {
        // Fond + bordure (accentuée si focalisé).
        let (border_color, border_width) = if status.focused {
            (BORDER_FOCUS, 2.0)
        } else {
            (BORDER_IDLE, 1.0)
        };
        scene.draw_rect(bounds, BG, 6.0, border_width, border_color);

        let text_x = bounds.x + PAD_X;
        let text_y = bounds.y + PAD_Y;
        if !self.value.is_empty() {
            scene.text(
                Point::new(text_x, text_y),
                self.value.clone(),
                self.size,
                TEXT_COLOR,
            );
        }

        // Curseur au bout du texte lorsqu'il est focalisé.
        if status.focused {
            let text_width = frus_text::measure(&self.value, self.size).width;
            scene.fill_rect(
                Rect::new(
                    text_x + text_width + 1.0,
                    text_y,
                    2.0,
                    frus_text::line_height(self.size),
                ),
                TEXT_COLOR,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn on_key(&self, key: &Key) -> Option<Msg> {
        let value = self.apply_key(key)?;
        self.on_input.as_ref().map(|make| make(value))
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
        Changed(String),
    }

    #[test]
    fn typing_appends_and_backspace_removes() {
        let input = TextInput::new("ab").on_input(Msg::Changed);

        assert_eq!(
            input.on_key(&Key::Text("c".to_string())),
            Some(Msg::Changed("abc".to_string()))
        );
        assert_eq!(
            input.on_key(&Key::Backspace),
            Some(Msg::Changed("a".to_string()))
        );
        // Entrée : aucune modification.
        assert_eq!(input.on_key(&Key::Enter), None);
    }

    #[test]
    fn is_focusable() {
        let input: TextInput<Msg> = TextInput::new("");
        assert!(Widget::<Msg>::focusable(&input));
    }
}
