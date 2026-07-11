//! [`Pagination`] : un sélecteur de page — ‹ préc. · fenêtre de pages · suiv. ›.

use frus_core::{Rect, Scene};
use frus_layout::{FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Nombre de pages affichées de part et d'autre de la page courante.
const WINDOW: usize = 2;

/// Un bouton de page (actif, lien, ou désactivé).
fn page_button<Msg: Clone + 'static>(
    label: impl Into<String>,
    message: Option<Msg>,
    active: bool,
) -> Box<dyn Widget<Msg>> {
    let variant = if active {
        Variant::Primary
    } else {
        Variant::Secondary
    };
    let mut button = Button::new(label).variant(variant).size(15.0);
    if let Some(message) = message {
        button = button.on_press(message);
    }
    Box::new(button)
}

/// Un sélecteur de page contrôlé (pages **1-indexées**).
pub struct Pagination<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Pagination<Msg> {
    /// Crée le sélecteur : page `current` sur `total`, `on_select(p)` au clic.
    pub fn new(current: usize, total: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        let total = total.max(1);
        let current = current.clamp(1, total);
        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();

        // ‹ précédent (désactivé sur la première page).
        children.push(page_button(
            "‹",
            (current > 1).then(|| on_select(current - 1)),
            false,
        ));

        // Fenêtre de pages autour de la courante.
        let start = current.saturating_sub(WINDOW).max(1);
        let end = (current + WINDOW).min(total);
        for page in start..=end {
            children.push(page_button(
                page.to_string(),
                Some(on_select(page)),
                page == current,
            ));
        }

        // › suivant (désactivé sur la dernière page).
        children.push(page_button(
            "›",
            (current < total).then(|| on_select(current + 1)),
            false,
        ));

        Self { children }
    }
}

impl<Msg: Clone> Widget<Msg> for Pagination<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 4.0,
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
        Page(usize),
    }

    #[test]
    fn windows_pages_and_bounds_prev_next() {
        // 10 pages, courante 5 → ‹ + [3 4 5 6 7] + › = 7 enfants.
        let p = Pagination::new(5, 10, Msg::Page);
        let children = Widget::<Msg>::children(&p);
        assert_eq!(children.len(), 7);
        // ‹ va à la page 4.
        assert_eq!(children[0].on_click(), Some(Msg::Page(4)));
        // › va à la page 6.
        assert_eq!(children[6].on_click(), Some(Msg::Page(6)));
    }

    #[test]
    fn first_page_disables_prev() {
        let p = Pagination::new(1, 3, Msg::Page);
        let children = Widget::<Msg>::children(&p);
        assert_eq!(children[0].on_click(), None); // ‹ désactivé
    }
}
