//! [`Transform`] : décale son enfant à la **peinture**, sans toucher la mise en
//! page (façon `Transform.translate` de Flutter).

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Décale tout le sous-arbre de son enfant de `(dx, dy)` pixels **au rendu et au
/// hit-test**, sans changer la mise en page : les frères ne bougent pas et
/// l'enfant peut déborder sa boîte (aucune découpe, comme Flutter).
///
/// C'est un décalage de *peinture* : combiné à un `Tween` lu dans `view()`, il
/// fait **glisser** le sous-arbre (pastille qui déborde un coin, entrée qui
/// coulisse, secousse d'erreur…). Seule la **translation** est gérée ici ;
/// l'échelle et la rotation (matrice) viendront plus tard.
pub struct Transform<Msg> {
    dx: f32,
    dy: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Transform<Msg> {
    /// Décale l'enfant de `(dx, dy)` pixels logiques (x vers la droite, y vers le
    /// bas), sans toucher la mise en page.
    pub fn translate(dx: f32, dy: f32) -> Self {
        Self {
            dx,
            dy,
            children: Vec::new(),
        }
    }

    /// Définit l'enfant décalé.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Transform<Msg> {
    fn style(&self) -> Style {
        // Passe-plat : la boîte prend sa taille du contexte comme l'enfant ; le
        // décalage n'agit qu'à la peinture (voir `transform_translate`).
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Widget de transformation pur : aucune décoration propre.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn transform_translate(&self) -> Option<(f32, f32)> {
        Some((self.dx, self.dy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;
    use frus_core::{Color, Primitive, Size};

    /// `Transform::translate(30, 10)` décale l'enfant (20×20) à la peinture : son
    /// fond, normalement en haut-gauche, est peint à ~(30, 10).
    #[test]
    fn translate_offsets_the_child_at_paint() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::translate(30.0, 10.0)
                .child(Container::new().width(20.0).height(20.0).color(red)),
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
            (rect.x - 30.0).abs() < 0.5 && (rect.y - 10.0).abs() < 0.5,
            "décalé à (30, 10) : {rect:?}"
        );
    }

    /// Le décalage est **purement visuel** : un frère placé après un enfant
    /// transformé garde sa position de mise en page (le Transform n'agrandit ni ne
    /// déplace sa boîte). Ici le 2e enfant reste à `y = 20`, malgré le décalage
    /// vertical de 50 du 1er.
    #[test]
    fn translate_does_not_affect_layout() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = crate::Flex::<()>::column()
            .width(100.0)
            .child(
                Transform::translate(0.0, 50.0)
                    .child(Container::new().flex(1.0).height(20.0).color(red)),
            )
            .child(Container::new().height(20.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let green_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => Some(rect.y),
                _ => None,
            })
            .expect("le fond vert du 2e enfant");
        // Le 1er enfant occupe 20px de haut en layout (son décalage de 50 est visuel) :
        // le 2e enfant suit à y = 20.
        assert!((green_y - 20.0).abs() < 0.5, "frère à sa place layout : y = {green_y}");
    }
}
