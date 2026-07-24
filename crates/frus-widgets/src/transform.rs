//! [`Transform`] : décale son enfant à la **peinture**, sans toucher la mise en
//! page (façon `Transform.translate` de Flutter).

use frus_core::{Alignment, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Transforme son enfant **à la peinture** (rendu **et** hit-test), sans changer
/// la mise en page : les frères ne bougent pas et l'enfant peut déborder sa boîte
/// (aucune découpe, comme Flutter).
///
/// Deux transformations, chacune combinable à un `Tween` lu dans `view()` pour
/// **animer** :
/// - **`translate(dx, dy)`** — décale le sous-arbre (pastille dans un coin, entrée
///   qui coulisse, secousse d'erreur…).
/// - **`scale(factor)`** — met le sous-arbre à l'échelle autour d'un pivot (par
///   défaut le centre) : effet « pop » d'un bouton, zoom d'une vignette. Reste
///   aligné sur les axes (un rect mis à l'échelle reste un rect) — aucune matrice.
/// - **`rotate(radians)`** — tourne le sous-arbre autour d'un pivot (par défaut le
///   centre) : aiguille, chevron qui bascule, spinner. Le sous-arbre est peint dans
///   un calque **composité tourné** ; le hit-test contre-tourne le point.
pub struct Transform<Msg> {
    dx: f32,
    dy: f32,
    /// `(facteur, pivot)` — `None` = pas d'échelle.
    scale: Option<(f32, Alignment)>,
    /// `(angle_radians, pivot)` — `None` = pas de rotation.
    rotate: Option<(f32, Alignment)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Transform<Msg> {
    /// Décale l'enfant de `(dx, dy)` pixels logiques (x vers la droite, y vers le
    /// bas), sans toucher la mise en page.
    pub fn translate(dx: f32, dy: f32) -> Self {
        Self {
            dx,
            dy,
            scale: None,
            rotate: None,
            children: Vec::new(),
        }
    }

    /// Met l'enfant à l'échelle par `factor` **autour de son centre**, sans toucher
    /// la mise en page (`1.0` = neutre, `2.0` = double, `0.5` = moitié).
    pub fn scale(factor: f32) -> Self {
        Self::scale_from(factor, Alignment::CENTER)
    }

    /// Comme [`Transform::scale`], mais autour d'un `pivot` (ancrage dans la boîte :
    /// `Alignment::TOP_LEFT` fixe le coin haut-gauche, etc.).
    pub fn scale_from(factor: f32, pivot: Alignment) -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            scale: Some((factor, pivot)),
            rotate: None,
            children: Vec::new(),
        }
    }

    /// Tourne l'enfant de `radians` (sens horaire) **autour de son centre**, sans
    /// toucher la mise en page.
    pub fn rotate(radians: f32) -> Self {
        Self::rotate_from(radians, Alignment::CENTER)
    }

    /// Comme [`Transform::rotate`], mais autour d'un `pivot` (ancrage dans la boîte).
    pub fn rotate_from(radians: f32, pivot: Alignment) -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            scale: None,
            rotate: Some((radians, pivot)),
            children: Vec::new(),
        }
    }

    /// Définit l'enfant transformé.
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
        ((self.dx != 0.0) || (self.dy != 0.0)).then_some((self.dx, self.dy))
    }

    fn transform_scale(&self) -> Option<(f32, Alignment)> {
        self.scale
    }

    fn transform_rotate(&self) -> Option<(f32, Alignment)> {
        self.rotate
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

    /// `Transform::scale(2.0)` double l'enfant (20×20) autour de **son centre** :
    /// le fond mesure ~40×40 et reste centré sur le même point (10, 10).
    #[test]
    fn scale_grows_the_child_about_its_center() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale(2.0).child(Container::new().width(20.0).height(20.0).color(red)),
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
            (rect.width - 40.0).abs() < 0.5 && (rect.height - 40.0).abs() < 0.5,
            "doublé : {rect:?}"
        );
        let (cx, cy) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        assert!(
            (cx - 10.0).abs() < 0.5 && (cy - 10.0).abs() < 0.5,
            "même centre (10, 10) : ({cx}, {cy})"
        );
    }

    /// `scale_from(2.0, TOP_LEFT)` met à l'échelle autour du coin haut-gauche de
    /// l'enfant : ce coin reste fixe à (0, 0) et le fond grandit vers le bas-droite.
    #[test]
    fn scale_from_pins_the_pivot_corner() {
        use frus_core::Alignment;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale_from(2.0, Alignment::TOP_LEFT)
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
            rect.x.abs() < 0.5 && rect.y.abs() < 0.5,
            "coin haut-gauche fixe à (0, 0) : {rect:?}"
        );
        assert!(
            (rect.width - 40.0).abs() < 0.5 && (rect.height - 40.0).abs() < 0.5,
            "doublé : {rect:?}"
        );
    }

    /// `Transform::rotate` enveloppe le sous-arbre dans un **calque tourné** portant
    /// l'angle et le pivot (centre de l'enfant 40×20 → (20, 10)).
    #[test]
    fn rotate_emits_a_rotated_layer() {
        use frus_core::{LayerTransform, Primitive};
        use std::f32::consts::FRAC_PI_2;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::rotate(FRAC_PI_2)
                .child(Container::new().width(40.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let t: LayerTransform = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer { transform: Some(t), .. } => Some(*t),
                _ => None,
            })
            .expect("un calque tourné");
        assert!((t.angle - FRAC_PI_2).abs() < 1e-3, "angle : {}", t.angle);
        assert!(
            (t.pivot.x - 20.0).abs() < 0.5 && (t.pivot.y - 10.0).abs() < 0.5,
            "pivot au centre de l'enfant : {:?}",
            t.pivot
        );
    }

    /// Le hit-test **contre-tourne** le point : un clic à la position *tournée* d'un
    /// enfant cliquable l'atteint, alors que sa position d'origine (non tournée) ne
    /// l'atteint plus. Enfant 40×20 tourné de +90° autour de (20, 10) : le point
    /// interne (35, 10) apparaît à l'écran en (20, 25).
    #[test]
    fn rotate_hit_test_counter_rotates_the_point() {
        use frus_core::Point;
        use std::f32::consts::FRAC_PI_2;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<i32>::column().width(100.0).child(
            Transform::rotate(FRAC_PI_2).child(
                Container::new().width(40.0).height(20.0).color(red).on_click(7),
            ),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        // À l'écran, le point interne (35, 10) est peint en (20, 25) après rotation.
        assert!(ui.hit(Point::new(20.0, 25.0)).is_some(), "clic sur la position tournée");
        // La position d'origine (non tournée) ne recouvre plus l'enfant.
        assert!(ui.hit(Point::new(35.0, 10.0)).is_none(), "l'ancienne position rate");
    }
}
