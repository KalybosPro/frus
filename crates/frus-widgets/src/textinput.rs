//! [`TextInput`] : un champ de saisie mono-ligne, **contrôlé** (sa valeur vient
//! de l'état applicatif), avec curseur, navigation et sélection.
//!
//! La valeur est contrôlée ; le **curseur / la sélection** sont un état d'édition
//! retenu au runtime ([`Edit`]), clé par identité de widget.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::{Key, Status};
use crate::runtime::Edit;
use crate::widget::Widget;

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 6.0;

const BG: Color = Color::rgb(0.16, 0.17, 0.20);
const TEXT_COLOR: Color = Color::rgb(0.92, 0.92, 0.95);
const BORDER_IDLE: Color = Color::rgb(0.32, 0.34, 0.40);
const BORDER_FOCUS: Color = Color::rgb(0.35, 0.62, 0.95);
const SELECTION: Color = Color::rgba(0.35, 0.62, 0.95, 0.4);

/// Un champ de saisie de texte sur une ligne.
pub struct TextInput<Msg> {
    value: String,
    size: f32,
    width: Dimension,
    on_input: Option<Box<dyn Fn(String) -> Msg>>,
}

/// Largeur en pixels des `count` premiers caractères de `chars`, à la taille donnée.
fn prefix_width(chars: &[char], count: usize, size: f32) -> f32 {
    let prefix: String = chars[..count.min(chars.len())].iter().collect();
    frus_text::measure(&prefix, size).width
}

/// Déplace le curseur vers `target`, en gérant l'ancre de sélection selon Shift.
fn move_cursor(cursor: &mut usize, anchor: &mut Option<usize>, target: usize, shift: bool) {
    if shift {
        if anchor.is_none() {
            *anchor = Some(*cursor);
        }
    } else {
        *anchor = None;
    }
    *cursor = target;
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
        let (border_color, border_width) = if status.focused {
            (BORDER_FOCUS, 2.0)
        } else {
            (BORDER_IDLE, 1.0)
        };
        scene.draw_rect(bounds, BG, 6.0, border_width, border_color);

        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let text_x = bounds.x + PAD_X;
        let text_y = bounds.y + PAD_Y;
        let line_h = frus_text::line_height(self.size);

        // Surbrillance de sélection (sous le texte).
        if status.focused {
            if let Some((start, end)) = status.selection {
                let (start, end) = (start.min(len), end.min(len));
                if start < end {
                    let x0 = text_x + prefix_width(&chars, start, self.size);
                    let x1 = text_x + prefix_width(&chars, end, self.size);
                    scene.fill_rect(Rect::new(x0, text_y, x1 - x0, line_h), SELECTION);
                }
            }
        }

        if !self.value.is_empty() {
            scene.text(
                Point::new(text_x, text_y),
                self.value.clone(),
                self.size,
                TEXT_COLOR,
            );
        }

        // Curseur.
        if status.focused {
            let cursor = status.cursor.unwrap_or(len).min(len);
            let cx = text_x + prefix_width(&chars, cursor, self.size);
            scene.fill_rect(Rect::new(cx, text_y, 2.0, line_h), TEXT_COLOR);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn on_edit(&self, edit: &mut Edit, key: &Key) -> Option<Msg> {
        let mut chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let mut cursor = edit.cursor.min(len);
        let mut anchor = edit.anchor.map(|a| a.min(len));
        let selection = anchor
            .map(|a| (a.min(cursor), a.max(cursor)))
            .filter(|(s, e)| s < e);

        let mut changed = false;

        match key {
            Key::Text(text) => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                }
                let inserted: Vec<char> = text.chars().collect();
                let n = inserted.len();
                chars.splice(cursor..cursor, inserted);
                cursor += n;
                anchor = None;
                changed = true;
            }
            Key::Backspace => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                    changed = true;
                } else if cursor > 0 {
                    chars.remove(cursor - 1);
                    cursor -= 1;
                    changed = true;
                }
                anchor = None;
            }
            Key::Delete => {
                if let Some((s, e)) = selection {
                    chars.drain(s..e);
                    cursor = s;
                    changed = true;
                } else if cursor < len {
                    chars.remove(cursor);
                    changed = true;
                }
                anchor = None;
            }
            Key::Left { shift } => {
                let target = cursor.saturating_sub(1);
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            Key::Right { shift } => {
                let target = (cursor + 1).min(len);
                move_cursor(&mut cursor, &mut anchor, target, *shift);
            }
            Key::Home { shift } => move_cursor(&mut cursor, &mut anchor, 0, *shift),
            Key::End { shift } => move_cursor(&mut cursor, &mut anchor, len, *shift),
            Key::Enter => {}
        }

        edit.cursor = cursor;
        edit.anchor = anchor;

        if changed {
            let new_value: String = chars.into_iter().collect();
            self.on_input.as_ref().map(|make| make(new_value))
        } else {
            None
        }
    }

    fn cursor_at(&self, local_x: f32) -> Option<usize> {
        let target = local_x - PAD_X;
        let chars: Vec<char> = self.value.chars().collect();
        let mut best = 0;
        let mut best_dist = f32::MAX;
        for i in 0..=chars.len() {
            let dist = (prefix_width(&chars, i, self.size) - target).abs();
            if dist < best_dist {
                best_dist = dist;
                best = i;
            }
        }
        Some(best)
    }

    fn selected_text(&self, edit: &Edit) -> Option<String> {
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let cursor = edit.cursor.min(len);
        let anchor = edit.anchor?.min(len);
        let (start, end) = (anchor.min(cursor), anchor.max(cursor));
        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
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

    fn input(value: &str) -> TextInput<Msg> {
        TextInput::new(value).on_input(Msg::Changed)
    }

    #[test]
    fn insert_at_cursor() {
        let inp = input("ac");
        let mut edit = Edit { cursor: 1, anchor: None };
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Text("b".to_string())),
            Some(Msg::Changed("abc".to_string()))
        );
        assert_eq!(edit.cursor, 2);
    }

    #[test]
    fn shift_arrow_selects_then_delete() {
        let inp = input("hello");
        // Curseur en fin, Shift+Left deux fois -> sélectionne "lo".
        let mut edit = Edit { cursor: 5, anchor: None };
        inp.on_edit(&mut edit, &Key::Left { shift: true });
        inp.on_edit(&mut edit, &Key::Left { shift: true });
        assert_eq!(edit.selection_range(), Some((3, 5)));
        // Backspace supprime la sélection.
        assert_eq!(
            inp.on_edit(&mut edit, &Key::Backspace),
            Some(Msg::Changed("hel".to_string()))
        );
        assert_eq!(edit.cursor, 3);
        assert_eq!(edit.anchor, None);
    }

    #[test]
    fn home_end_bounds() {
        let inp = input("abc");
        let mut edit = Edit { cursor: 1, anchor: None };
        inp.on_edit(&mut edit, &Key::End { shift: false });
        assert_eq!(edit.cursor, 3);
        inp.on_edit(&mut edit, &Key::Home { shift: false });
        assert_eq!(edit.cursor, 0);
    }

    #[test]
    fn selected_text_reads_range() {
        let inp = input("hello");
        let edit = Edit { cursor: 5, anchor: Some(2) };
        assert_eq!(inp.selected_text(&edit), Some("llo".to_string()));
    }
}
