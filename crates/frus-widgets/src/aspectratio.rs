//! [`AspectRatio`] : contraint son enfant à un **rapport largeur/hauteur** donné
//! (façon `AspectRatio` de Flutter).

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Impose un rapport `width / height` à sa boîte : la boîte **prend toute la
/// largeur disponible**, puis dérive sa hauteur du rapport (`hauteur = largeur /
/// rapport`). Dans une colonne large de 100, `AspectRatio::new(2.0)` fait une
/// boîte de 100×50 ; `AspectRatio::new(0.5)`, une boîte de 100×200.
///
/// Le rapport suit la convention de Flutter (et de taffy) : `largeur / hauteur`
/// — `2.0` est deux fois plus large que haut, `0.5` deux fois plus haut que large.
///
/// L'enfant hérite de la boîte : il s'étire en hauteur (axe croisé) et remplit la
/// largeur s'il grandit (`flex`) — typiquement une image ou un fond plein.
pub struct AspectRatio<Msg> {
    ratio: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> AspectRatio<Msg> {
    /// Crée une boîte au rapport `width / height` donné (borné à `> 0`).
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.max(f32::MIN_POSITIVE),
            children: Vec::new(),
        }
    }

    /// Définit l'enfant contraint au rapport.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for AspectRatio<Msg> {
    fn style(&self) -> Style {
        // La boîte prend toute la largeur du parent (dimension **connue** de
        // taffy) ; le rapport en dérive la hauteur. Une largeur seulement
        // « étirée » (align stretch) ne suffit pas à taffy pour dériver l'axe.
        Style {
            width: Dimension::Percent(1.0),
            aspect_ratio: Some(self.ratio),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Widget de disposition pur : aucune décoration propre.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;
    use frus_core::{Color, Primitive, Size};

    /// Dans une colonne large de 100, un `AspectRatio(2.0)` produit une boîte
    /// 100×50 (largeur pleine, hauteur dérivée) ; son enfant, qui remplit, peint
    /// un fond de ~100×50.
    #[test]
    fn derives_free_dimension_from_ratio() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            AspectRatio::new(2.0).child(Container::new().flex(1.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("le fond rouge de l'enfant");
        assert!(
            (rect.width - 100.0).abs() < 0.5 && (rect.height - 50.0).abs() < 0.5,
            "rapport 2.0 → 100×50 : {rect:?}"
        );
    }

    /// Un rapport < 1 rend la boîte **plus haute que large** : `0.5` dans une
    /// colonne de 100 → 100×200.
    #[test]
    fn ratio_below_one_is_taller_than_wide() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            AspectRatio::new(0.5).child(Container::new().flex(1.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 400.0), &rt, &theme);
        let rect = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 => Some(*rect),
                _ => None,
            })
            .expect("le fond rouge de l'enfant");
        assert!(
            (rect.width - 100.0).abs() < 0.5 && (rect.height - 200.0).abs() < 0.5,
            "rapport 0.5 → 100×200 : {rect:?}"
        );
    }
}
