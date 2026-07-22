//! [`FractionallySizedBox`] : dimensionne sa boîte à une **fraction** de l'espace
//! du parent (façon `FractionallySizedBox` de Flutter).

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Prend une **fraction** de la taille du parent sur chaque axe réglé :
/// `width_factor(0.5)` = moitié de la largeur du parent, `height_factor(0.25)` =
/// quart de sa hauteur. Un axe **non réglé** suit son contenu (taille naturelle).
/// L'enfant remplit la boîte (étirement / `flex`).
///
/// C'est l'équivalent, dans notre modèle flex, du `FractionallySizedBox` de
/// Flutter : plutôt que de contraindre l'enfant, la boîte **se dimensionne
/// elle-même** en pourcentage du parent (via `Dimension::Percent`), ce qui donne
/// le même résultat visuel dans le cas courant (enfant qui remplit).
pub struct FractionallySizedBox<Msg> {
    width_factor: Option<f32>,
    height_factor: Option<f32>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> FractionallySizedBox<Msg> {
    /// Crée une boîte fractionnaire sans facteur (les deux axes suivent le contenu
    /// tant qu'aucun n'est réglé).
    pub fn new() -> Self {
        Self {
            width_factor: None,
            height_factor: None,
            children: Vec::new(),
        }
    }

    /// Fraction `0.0..=1.0` de la **largeur** du parent (bornée à `>= 0`).
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.width_factor = Some(factor.max(0.0));
        self
    }

    /// Fraction `0.0..=1.0` de la **hauteur** du parent (bornée à `>= 0`).
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = Some(factor.max(0.0));
        self
    }

    /// Définit l'enfant, qui remplit la boîte fractionnaire.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for FractionallySizedBox<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for FractionallySizedBox<Msg> {
    fn style(&self) -> Style {
        // Un facteur réglé → dimension en pourcentage du parent ; sinon `Auto`
        // (l'axe suit le contenu).
        let dim = |factor: Option<f32>| match factor {
            Some(f) => Dimension::Percent(f),
            None => Dimension::Auto,
        };
        Style {
            width: dim(self.width_factor),
            height: dim(self.height_factor),
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

    /// `width_factor(0.5)` dans une colonne large de 100 → boîte large de 50 ;
    /// l'enfant qui remplit peint un fond de ~50 de large.
    #[test]
    fn width_factor_takes_a_fraction_of_the_parent() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            FractionallySizedBox::new()
                .width_factor(0.5)
                .child(Container::new().flex(1.0).height(20.0).color(red)),
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
        assert!((rect.width - 50.0).abs() < 0.5, "moitié de la largeur : {rect:?}");
    }

    /// `height_factor(0.25)` prend le quart de la hauteur du parent : dans une
    /// colonne haute de 200, la boîte fait 50 de haut.
    #[test]
    fn height_factor_takes_a_fraction_of_the_parent() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).height(200.0).child(
            FractionallySizedBox::new()
                .height_factor(0.25)
                .child(Container::new().flex(1.0).color(red)),
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
        assert!((rect.height - 50.0).abs() < 0.5, "quart de la hauteur : {rect:?}");
    }
}
