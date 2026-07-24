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
/// - **`scale(factor)`** / **`scale_xy(sx, sy)`** — met le sous-arbre à l'échelle
///   autour d'un pivot (par défaut le centre), uniforme ou **par axe** (étirer,
///   aplatir) : effet « pop » d'un bouton, zoom d'une vignette.
/// - **`rotate(radians)`** — tourne le sous-arbre autour d'un pivot (par défaut le
///   centre) : aiguille, chevron qui bascule, spinner.
///
/// Échelle et rotation (et leur composition) sont fondues en **une seule matrice
/// affine** portée par un calque composité transformé ; le hit-test applique la
/// matrice **inverse** au point. Elles se **composent** dans un même widget via les
/// enchaîneurs `and_translate`, `and_scale` / `and_scale_xy`, `and_rotate` — appliqués
/// dans l'ordre translation → échelle → rotation (la translation la plus intérieure,
/// via le décalage d'enfant). Ex. `Transform::scale(1.5).and_rotate(0.2)` grossit
/// **et** tourne, exactement, sans approximation de composition.
pub struct Transform<Msg> {
    dx: f32,
    dy: f32,
    /// `(sx, sy, pivot)` — `None` = pas d'échelle.
    scale: Option<(f32, f32, Alignment)>,
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
        Self::scale_xy_from(factor, factor, Alignment::CENTER)
    }

    /// Met l'enfant à l'échelle **par axe** (`sx` horizontal, `sy` vertical) autour
    /// de son centre — étirer ou aplatir. `scale_xy(2.0, 1.0)` double la largeur en
    /// gardant la hauteur.
    pub fn scale_xy(sx: f32, sy: f32) -> Self {
        Self::scale_xy_from(sx, sy, Alignment::CENTER)
    }

    /// Comme [`Transform::scale`], mais autour d'un `pivot` (ancrage dans la boîte :
    /// `Alignment::TOP_LEFT` fixe le coin haut-gauche, etc.).
    pub fn scale_from(factor: f32, pivot: Alignment) -> Self {
        Self::scale_xy_from(factor, factor, pivot)
    }

    /// Échelle par axe autour d'un `pivot` (la forme la plus générale).
    pub fn scale_xy_from(sx: f32, sy: f32, pivot: Alignment) -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            scale: Some((sx, sy, pivot)),
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

    /// **Ajoute** une translation à la transformation courante (composition) :
    /// `Transform::scale(1.2).and_translate(0, -4)` grossit *et* remonte.
    pub fn and_translate(mut self, dx: f32, dy: f32) -> Self {
        self.dx = dx;
        self.dy = dy;
        self
    }

    /// **Ajoute** une échelle uniforme (autour du centre) à la transformation
    /// courante.
    pub fn and_scale(self, factor: f32) -> Self {
        self.and_scale_xy(factor, factor)
    }

    /// **Ajoute** une échelle par axe (autour du centre) à la transformation courante.
    pub fn and_scale_xy(mut self, sx: f32, sy: f32) -> Self {
        self.scale = Some((sx, sy, Alignment::CENTER));
        self
    }

    /// **Ajoute** une rotation (autour du centre) à la transformation courante :
    /// `Transform::scale(1.5).and_rotate(0.2)` grossit *et* tourne.
    pub fn and_rotate(mut self, radians: f32) -> Self {
        self.rotate = Some((radians, Alignment::CENTER));
        self
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

    fn transform_scale(&self) -> Option<(f32, f32, Alignment)> {
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

    /// Extrait l'[`Affine`] du calque transformé de la scène (le sous-arbre est peint
    /// à plat *dans* ce calque ; la transformation est portée par sa matrice).
    fn layer_affine<Msg: Clone>(ui: &crate::ui::Ui<Msg>) -> frus_core::Affine {
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer { transform: Some(t), .. } => Some(t.affine),
                _ => None,
            })
            .expect("un calque transformé")
    }

    /// `Transform::scale(2.0)` porte une matrice qui double (partie linéaire = 2) et
    /// laisse **le centre de l'enfant (10, 10) fixe**.
    #[test]
    fn scale_grows_the_child_about_its_center() {
        use frus_core::Point;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale(2.0).child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!((m.m[0] - 2.0).abs() < 1e-3 && (m.m[3] - 2.0).abs() < 1e-3, "×2 : {:?}", m.m);
        let c = m.apply(Point::new(10.0, 10.0));
        assert!((c.x - 10.0).abs() < 0.5 && (c.y - 10.0).abs() < 0.5, "centre fixe : {c:?}");
    }

    /// `scale_from(2.0, TOP_LEFT)` met à l'échelle autour du coin haut-gauche de
    /// l'enfant : ce coin (0, 0) reste fixe.
    #[test]
    fn scale_from_pins_the_pivot_corner() {
        use frus_core::{Alignment, Point};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale_from(2.0, Alignment::TOP_LEFT)
                .child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!((m.m[0] - 2.0).abs() < 1e-3 && (m.m[3] - 2.0).abs() < 1e-3, "×2 : {:?}", m.m);
        let c = m.apply(Point::new(0.0, 0.0));
        assert!(c.x.abs() < 0.5 && c.y.abs() < 0.5, "coin haut-gauche fixe : {c:?}");
    }

    /// `Transform::scale_xy(3.0, 1.0)` porte une matrice d'échelle **par axe**
    /// (×3 en x, ×1 en y), centre fixe.
    #[test]
    fn scale_xy_stretches_per_axis() {
        use frus_core::Point;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(200.0).child(
            Transform::scale_xy(3.0, 1.0)
                .child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            (m.m[0] - 3.0).abs() < 1e-3 && (m.m[3] - 1.0).abs() < 1e-3,
            "×3 en x, ×1 en y : {:?}",
            m.m
        );
        let c = m.apply(Point::new(10.0, 10.0));
        assert!((c.x - 10.0).abs() < 0.5 && (c.y - 10.0).abs() < 0.5, "centre fixe : {c:?}");
    }

    /// `Transform::rotate(π/2)` porte une matrice de rotation pure (partie linéaire
    /// `[0, 1, -1, 0]`) qui laisse fixe le centre de l'enfant 40×20 → (20, 10).
    #[test]
    fn rotate_emits_a_rotated_layer() {
        use frus_core::Point;
        use std::f32::consts::FRAC_PI_2;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::rotate(FRAC_PI_2)
                .child(Container::new().width(40.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            m.m[0].abs() < 1e-3
                && (m.m[1] - 1.0).abs() < 1e-3
                && (m.m[2] + 1.0).abs() < 1e-3
                && m.m[3].abs() < 1e-3,
            "rotation +90° : {:?}",
            m.m
        );
        let c = m.apply(Point::new(20.0, 10.0));
        assert!((c.x - 20.0).abs() < 0.5 && (c.y - 10.0).abs() < 0.5, "centre fixe : {c:?}");
    }

    /// **Composition** : `scale(2.0).and_rotate(π/2)` fond les deux en **une seule**
    /// matrice = rotation ∘ échelle : la partie linéaire vaut `[0, 2, -2, 0]`
    /// (magnitude 2 = échelle, hors-diagonale = rotation).
    #[test]
    fn scale_and_rotate_compose() {
        use std::f32::consts::FRAC_PI_2;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = crate::Flex::<()>::column().width(100.0).child(
            Transform::scale(2.0)
                .and_rotate(FRAC_PI_2)
                .child(Container::new().width(20.0).height(20.0).color(red)),
        );
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(100.0, 200.0), &rt, &theme);
        let m = layer_affine(&ui);
        assert!(
            m.m[0].abs() < 1e-3
                && (m.m[1] - 2.0).abs() < 1e-3
                && (m.m[2] + 2.0).abs() < 1e-3
                && m.m[3].abs() < 1e-3,
            "rotation ∘ échelle : {:?}",
            m.m
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
