//! [`Tabs`] : une barre d'onglets **contrôlée** + le panneau sélectionné.
//!
//! Composite : ses enfants sont `[en-tête, panneau]` (colonne). Seul le contenu
//! de l'onglet sélectionné est réalisé (l'app reconstruit la vue à chaque frame).

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::flex::Flex;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Une vue à onglets.
pub struct Tabs<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Vec<String>,
    /// `[en-tête]` ou `[en-tête, panneau]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Tabs<Msg> {
    /// Crée des onglets : `selected` = index actif, `on_select(i)` = message au clic.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        let mut tabs = Self {
            selected,
            on_select: Box::new(on_select),
            labels: Vec::new(),
            children: Vec::new(),
        };
        tabs.rebuild_header();
        tabs
    }

    /// Ajoute un onglet (libellé + contenu). Le contenu n'est réalisé que s'il
    /// correspond à l'onglet sélectionné.
    pub fn tab(mut self, label: impl Into<String>, content: impl Widget<Msg> + 'static) -> Self {
        let index = self.labels.len();
        self.labels.push(label.into());
        self.rebuild_header();
        if index == self.selected {
            if self.children.len() > 1 {
                self.children[1] = Box::new(content);
            } else {
                self.children.push(Box::new(content));
            }
        }
        self
    }

    /// (Re)construit l'en-tête (boutons d'onglets) à l'index 0.
    fn rebuild_header(&mut self) {
        let mut header = Flex::row().gap(6.0);
        for (i, label) in self.labels.iter().enumerate() {
            let variant = if i == self.selected {
                Variant::Primary
            } else {
                Variant::Secondary
            };
            header = header.child(
                Button::new(label.clone())
                    .variant(variant)
                    .size(15.0)
                    .on_press((self.on_select)(i)),
            );
        }
        let header: Box<dyn Widget<Msg>> = Box::new(header);
        if self.children.is_empty() {
            self.children.push(header);
        } else {
            self.children[0] = header;
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Tabs<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            gap: 12.0,
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
        Select(usize),
    }

    #[test]
    fn shows_header_and_selected_panel() {
        let tabs = Tabs::new(1, Msg::Select)
            .tab("Un", Text::new("panneau un"))
            .tab("Deux", Text::new("panneau deux"))
            .tab("Trois", Text::new("panneau trois"));
        // [en-tête, panneau] — le panneau = contenu de l'onglet sélectionné (1).
        let children = Widget::<Msg>::children(&tabs);
        assert_eq!(children.len(), 2);
        // L'en-tête a 3 boutons.
        assert_eq!(children[0].children().len(), 3);
    }

    #[test]
    fn no_panel_when_selection_out_of_range() {
        let tabs = Tabs::new(9, Msg::Select).tab("Un", Text::new("x"));
        assert_eq!(Widget::<Msg>::children(&tabs).len(), 1); // en-tête seul
    }
}
