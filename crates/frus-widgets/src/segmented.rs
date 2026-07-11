//! [`SegmentedControl`] : un sélecteur segmenté **contrôlé** (boutons connectés,
//! sélection unique) — façon contrôle segmenté iOS.

use frus_core::{Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un contrôle segmenté à sélection unique.
pub struct SegmentedControl<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Vec<String>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SegmentedControl<Msg> {
    /// Crée un contrôle : `selected` = index actif, `on_select(i)` au clic.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            labels: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Ajoute un segment.
    pub fn segment(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        self.children = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let variant = if i == self.selected {
                    Variant::Primary
                } else {
                    Variant::Secondary
                };
                Box::new(
                    Button::new(label.clone())
                        .variant(variant)
                        .size(15.0)
                        .on_press((self.on_select)(i)),
                ) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for SegmentedControl<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 2.0,
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Select(usize),
    }

    #[test]
    fn segments_emit_index_and_highlight_selected() {
        let seg = SegmentedControl::new(1, Msg::Select)
            .segment("Jour")
            .segment("Semaine")
            .segment("Mois");
        let children = Widget::<Msg>::children(&seg);
        assert_eq!(children.len(), 3);
        // Cliquer le 3ᵉ segment → Select(2).
        assert_eq!(children[2].on_click(), Some(Msg::Select(2)));
    }
}
