//! [`InteractiveViewer`] : une fenêtre qui laisse **déplacer** (pan) et **zoomer**
//! son enfant, façon `InteractiveViewer` de Flutter. La transformation (échelle +
//! translation) est un **état retenu** dans le runtime, piloté par les gestes du
//! shell (glisser pour déplacer, molette / pincement pour zoomer).

use frus_core::{Affine, Point, Rect, Scene};

/// Friction du *fling* d'un pan relâché (décroissance exponentielle, par seconde).
pub(crate) const PAN_FRICTION: f32 = 6.0;
/// En deçà de cette vitesse (px/s), le fling s'arrête (au repos).
pub(crate) const PAN_MIN_VELOCITY: f32 = 24.0;
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// État **retenu** d'un [`InteractiveViewer`] : le facteur d'échelle et la
/// translation appliqués à l'enfant. Le point écran `q` reçoit le contenu peint à
/// plat en `p` selon `q = scale · p + (tx, ty)`. L'identité (`scale = 1`, translation
/// nulle) place l'enfant tel quel dans la fenêtre.
///
/// Toute la **mathématique des gestes** vit ici (pure, testable) : le shell se
/// contente d'appeler [`pan`](InteractiveView::pan) et
/// [`zoom_at`](InteractiveView::zoom_at) puis de restituer la [`matrix`](InteractiveView::matrix).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractiveView {
    /// Facteur d'échelle courant (`1.0` = taille naturelle).
    pub scale: f32,
    /// Translation de peinture, en px logiques.
    pub tx: f32,
    pub ty: f32,
}

impl Default for InteractiveView {
    fn default() -> Self {
        Self {
            scale: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }
}

impl InteractiveView {
    /// La matrice `q = scale · p + t` (échelle autour de l'origine puis translation).
    pub fn matrix(&self) -> Affine {
        Affine::scale(self.scale, self.scale).then(Affine::translation(self.tx, self.ty))
    }

    /// **Déplace** (pan) le contenu de `(dx, dy)` px écran : le doigt/curseur pousse
    /// le contenu du même delta.
    pub fn pan(self, dx: f32, dy: f32) -> Self {
        Self {
            tx: self.tx + dx,
            ty: self.ty + dy,
            ..self
        }
    }

    /// **Borne** la translation pour que le contenu (à l'échelle courante) **couvre**
    /// toujours la fenêtre `viewport` : on ne peut pas tirer un bord du contenu à
    /// l'intérieur de la fenêtre. Quand le contenu est **plus petit** que la fenêtre
    /// (dézoom sous 1), il est **centré**. À l'échelle 1, le pan est nul (le contenu
    /// remplit exactement).
    pub fn clamped(self, viewport: Rect) -> Self {
        // Contenu écran `q = s·p + t`, `p` couvrant la fenêtre à l'échelle 1. Bord
        // gauche du contenu ≤ bord gauche fenêtre → `t ≤ (1−s)·o` ; bord droit ≥ bord
        // droit fenêtre → `t ≥ (1−s)·(o+len)`. Pour `s < 1`, l'intervalle s'inverse →
        // on centre.
        let clamp_axis = |t: f32, s: f32, o: f32, len: f32| -> f32 {
            let hi = (1.0 - s) * o;
            let lo = (1.0 - s) * (o + len);
            if lo <= hi {
                t.clamp(lo, hi)
            } else {
                (lo + hi) * 0.5
            }
        };
        Self {
            scale: self.scale,
            tx: clamp_axis(self.tx, self.scale, viewport.x, viewport.width),
            ty: clamp_axis(self.ty, self.scale, viewport.y, viewport.height),
        }
    }

    /// **Zoome** d'un facteur `factor` en gardant fixe le point écran `cursor`
    /// (zoom ancré au curseur), l'échelle finale bornée à `[min, max]`. Le point du
    /// contenu sous le curseur ne bouge pas — le comportement attendu d'une molette
    /// ou d'un pincement.
    pub fn zoom_at(self, factor: f32, cursor: Point, min: f32, max: f32) -> Self {
        let new_scale = (self.scale * factor).clamp(min, max);
        // Facteur **effectif** après bornage (nul si déjà au bord).
        let f = new_scale / self.scale;
        // Fixe le point sous le curseur : t' = cursor·(1 - f) + f·t.
        Self {
            scale: new_scale,
            tx: cursor.x * (1.0 - f) + f * self.tx,
            ty: cursor.y * (1.0 - f) + f * self.ty,
        }
    }
}

/// Une fenêtre **déplaçable et zoomable** : son enfant remplit la fenêtre à
/// l'échelle 1, puis l'utilisateur le déplace (glisser) et le zoome (molette /
/// pincement) autour du curseur. Le contenu qui déborde est **découpé** à la fenêtre.
/// Idéal pour une carte, une image détaillée, un plan, un diagramme.
///
/// Comme [`crate::Scroll`], la fenêtre a besoin d'une **taille bornée** (sinon elle
/// s'effondre) : `width`/`height` fixes, ou `flex` dans une colonne/ligne. L'échelle
/// est bornée par `min_scale` / `max_scale`.
pub struct InteractiveViewer<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    min_scale: f32,
    max_scale: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> InteractiveViewer<Msg> {
    /// Une fenêtre interactive vide (échelle bornée à `0.5×`–`4×` par défaut).
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Length(300.0),
            flex_grow: 0.0,
            min_scale: 0.5,
            max_scale: 4.0,
            children: Vec::new(),
        }
    }

    /// Largeur fixe de la fenêtre (px logiques).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Hauteur fixe de la fenêtre (px logiques).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Échelle **minimale** autorisée (dézoom).
    pub fn min_scale(mut self, min: f32) -> Self {
        self.min_scale = min;
        self
    }

    /// Échelle **maximale** autorisée (zoom).
    pub fn max_scale(mut self, max: f32) -> Self {
        self.max_scale = max;
        self
    }

    /// Définit l'enfant déplaçable/zoomable.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }
}

impl<Msg> Default for InteractiveViewer<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for InteractiveViewer<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // La fenêtre est transparente : seul le contenu transformé est dessiné.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn interactive(&self) -> Option<(f32, f32)> {
        Some((self.min_scale, self.max_scale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'identité place le contenu tel quel : `matrix` est l'identité.
    #[test]
    fn default_is_identity() {
        let m = InteractiveView::default().matrix();
        let p = m.apply(Point::new(30.0, 40.0));
        assert!(
            (p.x - 30.0).abs() < 1e-4 && (p.y - 40.0).abs() < 1e-4,
            "identité : {p:?}"
        );
    }

    /// Le pan décale le contenu du delta exact.
    #[test]
    fn pan_shifts_the_content() {
        let v = InteractiveView::default().pan(12.0, -5.0);
        let p = v.matrix().apply(Point::new(0.0, 0.0));
        assert!(
            (p.x - 12.0).abs() < 1e-4 && (p.y + 5.0).abs() < 1e-4,
            "décalé : {p:?}"
        );
    }

    /// Le zoom garde **fixe le point sous le curseur** : ce point écran reçoit le
    /// même point du contenu avant et après le zoom.
    #[test]
    fn zoom_keeps_the_cursor_point_fixed() {
        let cursor = Point::new(100.0, 60.0);
        let before = InteractiveView::default();
        // Point du contenu actuellement sous le curseur (identité → = cursor).
        let content_under = {
            let inv = before.matrix().inverse();
            inv.apply(cursor)
        };
        let after = before.zoom_at(2.0, cursor, 0.5, 4.0);
        assert!((after.scale - 2.0).abs() < 1e-4, "×2 : {}", after.scale);
        // Ce même point du contenu doit se reprojeter **sur le curseur**.
        let reprojected = after.matrix().apply(content_under);
        assert!(
            (reprojected.x - cursor.x).abs() < 1e-3 && (reprojected.y - cursor.y).abs() < 1e-3,
            "point sous le curseur fixe : {reprojected:?}"
        );
    }

    /// Le zoom est **borné** : au-delà de `max`, l'échelle sature et le point sous
    /// le curseur reste fixe (facteur effectif nul).
    #[test]
    fn zoom_clamps_to_max() {
        let cursor = Point::new(50.0, 50.0);
        let v = InteractiveView {
            scale: 4.0,
            tx: 10.0,
            ty: 20.0,
        };
        let z = v.zoom_at(2.0, cursor, 0.5, 4.0);
        assert!((z.scale - 4.0).abs() < 1e-4, "saturé à max : {}", z.scale);
        assert!(
            (z.tx - 10.0).abs() < 1e-4 && (z.ty - 20.0).abs() < 1e-4,
            "inchangé au bord"
        );
    }

    /// À l'échelle 1, le pan est **annulé** par le bornage (le contenu remplit
    /// exactement la fenêtre).
    #[test]
    fn clamp_pins_pan_at_scale_one() {
        let vp = Rect::new(0.0, 0.0, 200.0, 200.0);
        let c = InteractiveView::default().pan(50.0, -30.0).clamped(vp);
        assert!(
            c.tx.abs() < 1e-4 && c.ty.abs() < 1e-4,
            "pan annulé à l'échelle 1 : {c:?}"
        );
    }

    /// Zoomé ×2, le pan est borné pour que le contenu **couvre** la fenêtre : un pan
    /// excessif est ramené au bord (le contenu ×2 s'étend sur `[t, t+400]`, il doit
    /// couvrir `[0, 200]` → `t ∈ [-200, 0]`).
    #[test]
    fn clamp_keeps_zoomed_content_covering() {
        let vp = Rect::new(0.0, 0.0, 200.0, 200.0);
        // Pan trop à droite (t positif) → ramené à 0 (bord gauche du contenu = 0).
        let a = InteractiveView {
            scale: 2.0,
            tx: 500.0,
            ty: 0.0,
        }
        .clamped(vp);
        assert!((a.tx - 0.0).abs() < 1e-4, "borné au bord gauche : {}", a.tx);
        // Pan trop à gauche → ramené à -200 (bord droit du contenu = 200).
        let b = InteractiveView {
            scale: 2.0,
            tx: -900.0,
            ty: 0.0,
        }
        .clamped(vp);
        assert!(
            (b.tx + 200.0).abs() < 1e-4,
            "borné au bord droit : {}",
            b.tx
        );
        // Un pan modéré passe inchangé.
        let c = InteractiveView {
            scale: 2.0,
            tx: -100.0,
            ty: -50.0,
        }
        .clamped(vp);
        assert!(
            (c.tx + 100.0).abs() < 1e-4 && (c.ty + 50.0).abs() < 1e-4,
            "pan valide conservé"
        );
    }

    /// Dézoomé (< 1), le contenu plus petit que la fenêtre est **centré**.
    #[test]
    fn clamp_centers_shrunken_content() {
        let vp = Rect::new(0.0, 0.0, 200.0, 200.0);
        let c = InteractiveView {
            scale: 0.5,
            tx: 999.0,
            ty: -999.0,
        }
        .clamped(vp);
        // Centre : t = (1 − 0.5)·(o + len/2) = 0.5·100 = 50 sur chaque axe.
        assert!(
            (c.tx - 50.0).abs() < 1e-4 && (c.ty - 50.0).abs() < 1e-4,
            "centré : {c:?}"
        );
    }

    /// La marche enveloppe l'enfant dans **un calque transformé et découpé à la
    /// fenêtre** : un `Primitive::Layer` porteur d'une matrice et d'un clip = viewport.
    #[test]
    fn walk_emits_a_transformed_clipped_layer() {
        use crate::Container;
        use frus_core::{Color, Primitive, Size};
        let root = InteractiveViewer::<()>::new()
            .width(200.0)
            .height(200.0)
            .child(Container::new().color(Color::rgb(1.0, 0.0, 0.0)));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        let (has_xform, clip) = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Layer {
                    transform, clip, ..
                } => Some((transform.is_some(), *clip)),
                _ => None,
            })
            .expect("un calque interactif");
        assert!(has_xform, "le calque porte la matrice de transformation");
        assert!(
            clip.width <= 200.5 && clip.height <= 200.5,
            "découpé à la fenêtre : {clip:?}"
        );
    }

    /// Régression : un frère placé **après** une fenêtre interactive garde sa place
    /// de mise en page. La fenêtre est une **feuille** de layout (son sous-arbre est
    /// posé à part) ; sans cela, l'index des rectangles se désynchronise et tout ce
    /// qui suit se superpose. Ici la fenêtre fait 150 de haut → le frère suit à y=150.
    #[test]
    fn sibling_after_viewer_keeps_its_layout_position() {
        use crate::{Container, Flex};
        use frus_core::{Color, Primitive, Size};
        let green = Color::rgb(0.0, 1.0, 0.0);
        let root = Flex::<()>::column()
            .width(300.0)
            .child(
                InteractiveViewer::new()
                    .width(300.0)
                    .height(150.0)
                    .child(Container::new().color(Color::rgb(0.2, 0.2, 0.2))),
            )
            .child(Container::new().width(300.0).height(20.0).color(green));
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(300.0, 600.0), &rt, &theme);
        let marker_y = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.5 && color.r < 0.5 => {
                    Some(rect.y)
                }
                _ => None,
            })
            .expect("le marqueur vert");
        assert!(
            (marker_y - 150.0).abs() < 0.5,
            "le frère suit la fenêtre (150 de haut), pas superposé : y = {marker_y}"
        );
    }

    /// Le hit-test **traverse la transformation** : après un pan de +50 en x, un clic
    /// à la position déplacée atteint l'enfant, et son ancienne position le rate.
    #[test]
    fn walk_pan_shifts_the_hit_test() {
        use crate::interaction::WidgetId;
        use crate::Container;
        use frus_core::{Color, Size};
        let root = InteractiveViewer::<i32>::new()
            .width(200.0)
            .height(200.0)
            .child(
                Container::new()
                    .width(200.0)
                    .height(200.0)
                    .color(Color::rgb(1.0, 0.0, 0.0))
                    .on_click(9),
            );
        let theme = crate::Theme::dark();

        // Identité : le bord gauche (x = 10) atteint l'enfant.
        let rt = crate::runtime::Runtime::default();
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        assert!(
            ui.hit(Point::new(10.0, 100.0)).is_some(),
            "identité : bord gauche atteint"
        );

        // Après pan +50 en x : le contenu est poussé à droite ; x = 10 tombe hors du
        // contenu (M⁻¹ = -40), mais x = 60 y retombe (M⁻¹ = 10).
        let mut rt = crate::runtime::Runtime::default();
        rt.interactive
            .insert(WidgetId::ROOT, InteractiveView::default().pan(50.0, 0.0));
        let ui = crate::ui::build_ui(&root, Size::new(200.0, 200.0), &rt, &theme);
        assert!(
            ui.hit(Point::new(10.0, 100.0)).is_none(),
            "pan : ancienne position ratée"
        );
        assert!(
            ui.hit(Point::new(60.0, 100.0)).is_some(),
            "pan : position déplacée atteinte"
        );
    }
}
