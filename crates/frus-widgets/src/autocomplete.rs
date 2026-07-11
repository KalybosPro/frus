//! [`Autocomplete`] : un champ de saisie avec une **liste de suggestions**
//! flottante. Contrôlé : l'application fournit la valeur **et** les suggestions
//! (déjà filtrées) ; la liste ne flotte que si elle est non vide.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::textinput::TextInput;
use crate::theme::Theme;
use crate::widget::Widget;

const WIDTH: f32 = 260.0;
const ROW_H: f32 = 32.0;
const PAD_X: f32 = 10.0;
const SIZE: f32 = 16.0;

/// Une suggestion cliquable.
struct Suggestion<Msg> {
    label: String,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Suggestion<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(WIDTH),
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let bg = theme.surface.lerp(theme.on_surface, 0.07 * status.hover_progress);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.border.fade(o));
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(
            Point::new(bounds.x + PAD_X, ty),
            self.label.clone(),
            SIZE,
            theme.on_surface.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Un champ de saisie avec suggestions.
pub struct Autocomplete<Msg> {
    on_pick: Box<dyn Fn(String) -> Msg>,
    labels: Vec<String>,
    /// `[champ]` ou `[champ, liste]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Autocomplete<Msg> {
    /// Crée un champ : valeur courante, `on_input(texte)` à la frappe, et
    /// `on_pick(suggestion)` au choix d'une suggestion.
    pub fn new(
        value: impl Into<String>,
        on_input: impl Fn(String) -> Msg + 'static,
        on_pick: impl Fn(String) -> Msg + 'static,
    ) -> Self {
        let input = TextInput::new(value).width(WIDTH).on_input(on_input);
        Self {
            on_pick: Box::new(on_pick),
            labels: Vec::new(),
            children: vec![Box::new(input)],
        }
    }

    /// Ajoute une suggestion à la liste flottante.
    pub fn suggestion(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        if self.labels.is_empty() {
            self.children.truncate(1);
            return;
        }
        let mut list = Flex::column().gap(2.0);
        for label in &self.labels {
            list = list.child(Suggestion {
                label: label.clone(),
                message: (self.on_pick)(label.clone()),
            });
        }
        let list: Box<dyn Widget<Msg>> = Box::new(list);
        if self.children.len() > 1 {
            self.children[1] = list;
        } else {
            self.children.push(list);
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Autocomplete<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
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

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children
            .get(1)
            .map(|list| (list.as_ref(), Placement::Below))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Input(String),
        Pick(String),
    }

    #[test]
    fn no_suggestions_no_overlay() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick);
        assert!(Widget::<Msg>::overlay(&ac).is_none());
    }

    #[test]
    fn suggestions_float_and_pick() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick)
            .suggestion("apple")
            .suggestion("apricot");
        assert!(Widget::<Msg>::overlay(&ac).is_some());
        // La liste contient les deux suggestions ; la 1ʳᵉ émet Pick("apple").
        let list = &Widget::<Msg>::children(&ac)[1];
        assert_eq!(list.children().len(), 2);
        assert_eq!(list.children()[0].on_click(), Some(Msg::Pick("apple".to_string())));
    }
}
