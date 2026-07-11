//! [`TextInput`] : un champ de saisie mono-ligne, **contrôlé** (sa valeur vient
//! de l'état applicatif), avec curseur, navigation et sélection.
//!
//! La valeur est contrôlée ; le **curseur / la sélection** sont un état d'édition
//! retenu au runtime ([`Edit`]), clé par identité de widget.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::{Key, Status};
use crate::runtime::Edit;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 6.0;

/// Un champ de saisie de texte sur une ligne.
pub struct TextInput<Msg> {
    value: String,
    size: f32,
    width: Dimension,
    on_input: Option<Box<dyn Fn(String) -> Msg>>,
    on_submit: Option<Msg>,
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
            on_submit: None,
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

    /// Message émis à la validation (touche Entrée), sans modifier la valeur.
    pub fn on_submit(mut self, message: Msg) -> Self {
        self.on_submit = Some(message);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for TextInput<Msg> {
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

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Bordure animée selon la progression de focus (repos → focus).
        let fp = status.focus_progress.clamp(0.0, 1.0);
        let border_color = theme.border.lerp(theme.focus, fp).fade(o);
        let border_width = 1.0 + fp;
        scene.draw_rect(bounds, theme.surface.fade(o), theme.radius, border_width, border_color);

        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        let content_x = bounds.x + PAD_X;
        let content_w = (bounds.width - PAD_X * 2.0).max(0.0);
        let text_y = bounds.y + PAD_Y;
        let line_h = frus_text::line_height(self.size);

        // Défilement horizontal : garde le curseur visible quand le texte dépasse.
        let scroll = if status.focused {
            let cursor = status.cursor.unwrap_or(len).min(len);
            (prefix_width(&chars, cursor, self.size) - content_w).max(0.0)
        } else {
            0.0
        };
        let text_x = content_x - scroll;

        // Découpe au cadre de contenu (sinon le texte déborde sur les voisins).
        let content_clip = scene
            .current_clip()
            .intersect(Rect::new(content_x, bounds.y, content_w, bounds.height));
        scene.set_clip(content_clip);

        // Surbrillance de sélection (sous le texte).
        if status.focused {
            if let Some((start, end)) = status.selection {
                let (start, end) = (start.min(len), end.min(len));
                if start < end {
                    let x0 = text_x + prefix_width(&chars, start, self.size);
                    let x1 = text_x + prefix_width(&chars, end, self.size);
                    scene.fill_rect(Rect::new(x0, text_y, x1 - x0, line_h), theme.selection.fade(o));
                }
            }
        }

        if !self.value.is_empty() {
            scene.text(
                Point::new(text_x, text_y),
                self.value.clone(),
                self.size,
                theme.on_surface.fade(o),
            );
        }

        // Curseur.
        if status.focused {
            let cursor = status.cursor.unwrap_or(len).min(len);
            let cx = text_x + prefix_width(&chars, cursor, self.size);
            scene.fill_rect(Rect::new(cx, text_y, 2.0, line_h), theme.on_surface.fade(o));
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
            Key::Enter => {
                // Validation : n'altère pas la valeur, émet le message de soumission.
                edit.cursor = cursor;
                edit.anchor = anchor;
                return self.on_submit.clone();
            }
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

    fn word_at(&self, index: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        if len == 0 {
            return Some((0, 0));
        }
        let i = index.min(len - 1);
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        if !is_word(chars[i]) {
            return Some((i, (i + 1).min(len)));
        }
        let mut start = i;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = i + 1;
        while end < len && is_word(chars[end]) {
            end += 1;
        }
        Some((start, end))
    }

    fn focusable(&self) -> bool {
        true
    }

    fn draws_own_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Changed(String),
        Submitted,
    }

    fn input(value: &str) -> TextInput<Msg> {
        TextInput::new(value).on_input(Msg::Changed)
    }

    #[test]
    fn text_is_clipped_to_content_box() {
        // Un texte plus long que le champ doit être découpé à sa largeur de contenu
        // (sinon il déborde sur les widgets voisins).
        let inp = input("a very long value that overflows the field width").width(100.0);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &inp,
            Rect::new(0.0, 0.0, 100.0, 30.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let clip = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { clip, .. } => Some(*clip),
                _ => None,
            })
            .expect("primitive de texte");
        assert!((clip.width - (100.0 - PAD_X * 2.0)).abs() < 0.5, "découpe = {clip:?}");
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

    #[test]
    fn enter_submits_without_changing_value() {
        let inp = input("acheter du lait").on_submit(Msg::Submitted);
        let mut edit = Edit { cursor: 3, anchor: None };
        // Entrée : émet la soumission, ne renvoie pas de changement de valeur.
        assert_eq!(inp.on_edit(&mut edit, &Key::Enter), Some(Msg::Submitted));
        assert_eq!(edit.cursor, 3); // curseur inchangé
    }

    #[test]
    fn enter_without_submit_is_noop() {
        let inp = input("x");
        let mut edit = Edit { cursor: 1, anchor: None };
        assert_eq!(inp.on_edit(&mut edit, &Key::Enter), None);
    }

    #[test]
    fn word_at_finds_word_bounds() {
        let inp = input("bonjour le monde");
        // index 10 tombe dans "monde" (indices 10..15) — non, "le" est 8..10.
        assert_eq!(inp.word_at(2), Some((0, 7))); // "bonjour"
        assert_eq!(inp.word_at(11), Some((11, 16))); // "monde"
    }
}
