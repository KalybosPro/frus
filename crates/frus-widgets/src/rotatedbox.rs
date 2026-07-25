//! [`RotatedBox`] : tourne son enfant d'un **quart de tour** entier — et, à la
//! différence de [`crate::Transform`], la rotation **affecte la mise en page** (la
//! boîte échange largeur et hauteur pour un nombre impair de quarts), façon
//! `RotatedBox` de Flutter.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Tourne son enfant de `quarter_turns` **quarts de tour** (90° chacun, sens
/// horaire), **en changeant la mise en page** : pour un nombre impair de quarts, la
/// boîte présentée au parent a la **largeur et la hauteur de l'enfant échangées** (un
/// libellé vertical dans une barre latérale, une étiquette de graphe tournée…).
///
/// L'enfant est mesuré à sa taille **naturelle**, centré, puis tourné autour du
/// centre de la boîte — le hit-test contre-tourne le point (comme `Transform`). Un
/// `quarter_turns` négatif tourne dans l'autre sens ; seul le reste modulo 4 compte.
pub struct RotatedBox<Msg> {
    quarter_turns: i32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> RotatedBox<Msg> {
    /// Tourne l'enfant de `quarter_turns` quarts de tour horaires.
    pub fn new(quarter_turns: i32) -> Self {
        Self { quarter_turns, children: Vec::new() }
    }

    /// Définit l'enfant tourné.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for RotatedBox<Msg> {
    fn style(&self) -> Style {
        // La boîte réelle (dimensions échangées pour un quart impair) est calculée au
        // layout à partir de la taille naturelle de l'enfant (voir `build_layout`).
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Widget de rotation pur : aucune décoration propre.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn rotated_quarter_turns(&self) -> Option<i32> {
        Some(self.quarter_turns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Flex};
    use frus_core::{Color, Primitive, Size};

    /// Un quart de tour **échange** la largeur et la hauteur de la boîte : un enfant
    /// 80×20 occupe une boîte 20×80 dans la colonne (le frère suivant part de y=80).
    #[test]
    fn odd_quarter_turn_swaps_the_layout_box() {
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = Flex::<()>::column()
            .child(RotatedBox::new(1).child(Container::new().width(80.0).height(20.0).color(Color::rgb(0.3, 0.3, 0.3))))
            .child(Container::new().width(40.0).height(30.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let sibling_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => Some(rect.y),
                _ => None,
            })
            .expect("le frère vert");
        assert!(
            (sibling_y - 80.0).abs() < 0.5,
            "boîte tournée = 20×80 → le frère suit à y=80 : {sibling_y}"
        );
    }

    /// Un demi-tour (2 quarts) **ne change pas** les dimensions de la boîte.
    #[test]
    fn even_quarter_turn_keeps_the_box() {
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = Flex::<()>::column()
            .child(RotatedBox::new(2).child(Container::new().width(80.0).height(20.0).color(Color::rgb(0.3, 0.3, 0.3))))
            .child(Container::new().width(40.0).height(30.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let sibling_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => Some(rect.y),
                _ => None,
            })
            .expect("le frère vert");
        assert!((sibling_y - 20.0).abs() < 0.5, "boîte inchangée 80×20 → frère à y=20 : {sibling_y}");
    }

    /// La rotation émet un **calque tourné** (partie linéaire hors-diagonale).
    #[test]
    fn emits_a_rotated_layer() {
        let root = RotatedBox::<()>::new(1)
            .child(Container::new().width(80.0).height(20.0).color(Color::rgb(0.3, 0.3, 0.3)));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 400.0), &rt, &theme);
        let m = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer { transform: Some(t), .. } => Some(t.affine),
                _ => None,
            })
            .expect("un calque tourné");
        // +90° : partie linéaire ≈ [0, 1, -1, 0].
        assert!(m.m[0].abs() < 1e-3 && (m.m[1] - 1.0).abs() < 1e-3, "rotation +90° : {:?}", m.m);
    }
}
