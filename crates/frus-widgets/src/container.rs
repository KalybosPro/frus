//! [`Container`] : une boîte décorée (taille, marge, couleur, coins arrondis,
//! bordure, clic) avec un enfant optionnel.

use frus_core::{
    Border, BorderRadius, BoxDecoration, BoxShadow, Color, Curve, Insets, LinearGradient, Rect,
    Scene,
};
use frus_layout::{Dimension, Style};

use crate::interaction::{Interaction, Status};
use crate::theme::Theme;
use crate::widget::Widget;

/// Courbe d'easing (smoothstep) pour adoucir les transitions.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Une boîte rectangulaire décorée.
pub struct Container<Msg> {
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    padding: Insets,
    radius: BorderRadius,
    border_width: f32,
    border_color: Color,
    color: Option<Color>,
    hover_color: Option<Color>,
    pressed_color: Option<Color>,
    /// Dégradé : (couleur de fin, direction en espace `[0,1]²`).
    gradient: Option<(Color, [f32; 2])>,
    /// Ombre : (dx, dy, flou, couleur).
    shadow: Option<(f32, f32, f32, Color)>,
    on_click: Option<Msg>,
    on_long_press: Option<Msg>,
    /// Frontière de repaint : met en cache le sous-arbre peint (voir
    /// [`crate::Widget::repaint_boundary`]).
    repaint_boundary: bool,
    /// Opacité de **groupe** `[0,1]` appliquée au sous-arbre entier (façon
    /// `Opacity` de Flutter). `None` = opaque.
    opacity: Option<f32>,
    /// Si l'opacité de groupe est **animée** : `(durée, courbe)` de la transition
    /// (façon `AnimatedOpacity`). `None` = opacité fixe (pas de transition).
    opacity_anim: Option<(f32, Curve)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Container<Msg> {
    /// Crée un conteneur vide (taille automatique, sans décoration).
    pub fn new() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            padding: Insets::ZERO,
            radius: BorderRadius::ZERO,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            color: None,
            hover_color: None,
            pressed_color: None,
            gradient: None,
            shadow: None,
            on_click: None,
            on_long_press: None,
            repaint_boundary: false,
            opacity: None,
            opacity_anim: None,
            children: Vec::new(),
        }
    }

    /// Marque ce conteneur comme **frontière de repaint** : son sous-arbre est
    /// mis en cache et réutilisé tant que sa géométrie et l'état d'interaction
    /// de ses descendants sont stables. À poser autour de contenu **statique**
    /// qui, sinon, serait repeint à chaque frame d'animation voisine.
    pub fn repaint_boundary(mut self) -> Self {
        self.repaint_boundary = true;
        self
    }

    /// Fixe la largeur, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Fixe la hauteur, en pixels logiques.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Facteur d'expansion flex sur l'axe principal du parent.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Marge intérieure uniforme, en pixels logiques.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::uniform(padding);
        self
    }

    /// Marge intérieure par côté (haut, droite, bas, gauche).
    pub fn padding_each(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Insets::new(top, right, bottom, left);
        self
    }

    /// Rayons des coins arrondis : uniforme via `f32` (`.radius(10.0)`) ou par
    /// coin via [`BorderRadius`] (`.radius(BorderRadius::top(12.0))`).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Bordure : épaisseur (px) et couleur.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    /// Couleur de fond au repos.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Couleur de fond au survol.
    pub fn hover_color(mut self, color: Color) -> Self {
        self.hover_color = Some(color);
        self
    }

    /// Couleur de fond lorsqu'il est pressé.
    pub fn pressed_color(mut self, color: Color) -> Self {
        self.pressed_color = Some(color);
        self
    }

    /// Dégradé linéaire du fond (`color` → `end`), `dir` en espace `[0,1]²`
    /// (p. ex. `[0.0, 1.0]` = haut→bas).
    pub fn gradient(mut self, end: Color, dir: [f32; 2]) -> Self {
        self.gradient = Some((end, dir));
        self
    }

    /// Ombre portée : décalage `(dx, dy)`, rayon de flou et couleur.
    pub fn shadow(mut self, dx: f32, dy: f32, blur: f32, color: Color) -> Self {
        self.shadow = Some((dx, dy, blur, color));
        self
    }

    /// Message émis lorsque le conteneur est cliqué.
    pub fn on_click(mut self, message: Msg) -> Self {
        self.on_click = Some(message);
        self
    }

    /// Message émis par un **appui long** (pression maintenue sans mouvement).
    /// L'appui long évince le clic.
    pub fn on_long_press(mut self, message: Msg) -> Self {
        self.on_long_press = Some(message);
        self
    }

    /// Définit l'enfant du conteneur.
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        self.children.clear();
        self.children.push(Box::new(child));
        self
    }

    /// Applique une **opacité de groupe** `[0,1]` à tout le sous-arbre, d'un bloc
    /// (façon `Opacity` de Flutter) : le rendu passe par un calque composité, donc
    /// pas de double-superposition sur les chevauchements. `1.0` = aucun effet.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity.clamp(0.0, 1.0));
        self
    }

    /// Comme [`Container::opacity`], mais l'opacité **s'anime** vers `opacity` à
    /// chaque changement (façon `AnimatedOpacity`), avec `duration` (secondes) et
    /// `curve`. Le fondu porte sur le groupe entier.
    pub fn animated_opacity(mut self, opacity: f32, duration: f32, curve: Curve) -> Self {
        self.opacity = Some(opacity.clamp(0.0, 1.0));
        self.opacity_anim = Some((duration, curve));
        self
    }
}

impl<Msg> Default for Container<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone> Widget<Msg> for Container<Msg> {
    fn style(&self) -> Style {
        // La bordure **réserve sa place** dans la mise en page (comme le
        // `content_padding` d'une décoration) : le contenu d'une boîte bordée
        // n'est pas mangé par le trait.
        let mut padding = self.padding;
        if Border::new(self.border_width, self.border_color).is_visible() {
            padding.top += self.border_width;
            padding.right += self.border_width;
            padding.bottom += self.border_width;
            padding.left += self.border_width;
        }
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            padding,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, _theme: &Theme, scene: &mut Scene) {
        // Pressé : instantané. Sinon, transition animée repos → survol.
        let color = if status.interaction == Interaction::Pressed {
            self.pressed_color.or(self.hover_color).or(self.color)
        } else if let (Some(base), Some(hover)) = (self.color, self.hover_color) {
            Some(base.lerp(hover, ease(status.hover_progress)))
        } else {
            self.color
        };

        // Compose la décoration (fond/dégradé/bordure/ombre) et l'abaisse en
        // primitives dans l'ordre fixe ombre → fond → bordure. L'opacité (fondu
        // d'apparition) module toutes les couleurs.
        let decoration = BoxDecoration {
            color,
            gradient: self
                .gradient
                .map(|(end, dir)| LinearGradient::new(end, dir)),
            border: (self.border_width > 0.0)
                .then(|| Border::new(self.border_width, self.border_color)),
            radius: self.radius,
            shadow: self
                .shadow
                .map(|(dx, dy, blur, c)| BoxShadow::new(dx, dy, blur, c)),
        };
        decoration.paint_into(scene, bounds, status.opacity);
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }

    fn on_long_press(&self) -> Option<Msg> {
        self.on_long_press.clone()
    }

    fn repaint_boundary(&self) -> bool {
        self.repaint_boundary
    }

    fn opacity_group(&self) -> Option<f32> {
        self.opacity
    }

    /// Cible de l'opacité animée (uniquement si `animated_opacity` est posée) —
    /// c'est cette valeur que le runtime tween et que la marche relit pour le
    /// calque.
    fn anim_target(&self) -> Option<f32> {
        self.opacity_anim.as_ref().and(self.opacity)
    }

    fn anim_duration(&self) -> f32 {
        self.opacity_anim
            .as_ref()
            .map(|(d, _)| *d)
            .unwrap_or(crate::runtime::ANIM_DURATION)
    }

    fn anim_curve(&self) -> Curve {
        self.opacity_anim
            .as_ref()
            .map(|(_, c)| c.clone())
            .unwrap_or(Curve::Linear)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un `Container` avec opacité de groupe < 1 fait envelopper son sous-arbre
    /// peint dans un [`frus_core::Primitive::Layer`] à cette opacité.
    #[test]
    fn opacity_group_wraps_subtree_in_a_layer() {
        use frus_core::{Primitive, Size};
        let root: Container<()> = Container::new()
            .width(40.0)
            .height(40.0)
            .color(Color::rgb(1.0, 0.0, 0.0))
            .opacity(0.5);
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(64.0, 64.0), &rt, &theme);
        let layer = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Layer { opacity, primitives, .. } => Some((*opacity, primitives.len())),
            _ => None,
        });
        let (op, n) = layer.expect("un calque d'opacité de groupe");
        assert!((op - 0.5).abs() < 1e-6, "opacité de groupe = {op}");
        assert!(n >= 1, "le calque enveloppe le contenu peint ({n} primitives)");
    }

    /// Opacité pleine (`1.0`) : aucun calque n'est émis (chemin opaque, coût nul).
    #[test]
    fn full_opacity_emits_no_layer() {
        use frus_core::{Primitive, Size};
        let root: Container<()> = Container::new()
            .width(40.0)
            .height(40.0)
            .color(Color::rgb(1.0, 0.0, 0.0))
            .opacity(1.0);
        let rt = crate::runtime::Runtime::default();
        let theme = crate::Theme::dark();
        let ui = crate::ui::build_ui(&root, Size::new(64.0, 64.0), &rt, &theme);
        assert!(
            !ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Layer { .. })),
            "aucun calque à opacité pleine"
        );
    }

    /// `animated_opacity` déclare une valeur animée (le runtime la tween) avec la
    /// durée et la courbe fournies ; `opacity` seule non (opacité fixe).
    #[test]
    fn animated_opacity_declares_anim_target() {
        let animated: Container<()> =
            Container::new().animated_opacity(0.0, 0.3, Curve::ease_in());
        assert_eq!(Widget::<()>::anim_target(&animated), Some(0.0));
        assert_eq!(Widget::<()>::anim_duration(&animated), 0.3);
        assert_eq!(Widget::<()>::anim_curve(&animated), Curve::ease_in());
        assert_eq!(Widget::<()>::opacity_group(&animated), Some(0.0));

        // Opacité fixe : groupe oui, mais pas de valeur animée.
        let fixed: Container<()> = Container::new().opacity(0.5);
        assert_eq!(Widget::<()>::anim_target(&fixed), None);
        assert_eq!(Widget::<()>::opacity_group(&fixed), Some(0.5));
    }

    #[test]
    fn visible_border_reserves_layout_padding() {
        // Bordure visible : le padding de mise en page réserve son épaisseur.
        let bordered: Container<()> = Container::new().padding(4.0).border(2.0, Color::WHITE);
        assert_eq!(Widget::style(&bordered).padding, Insets::uniform(6.0));

        // Sans bordure (ou invisible) : padding inchangé.
        let plain: Container<()> = Container::new().padding(4.0);
        assert_eq!(Widget::style(&plain).padding, Insets::uniform(4.0));
        let invisible: Container<()> =
            Container::new().padding(4.0).border(2.0, Color::TRANSPARENT);
        assert_eq!(Widget::style(&invisible).padding, Insets::uniform(4.0));
    }
}
