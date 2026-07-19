//! [`Container`] : une boîte décorée (taille, marge, couleur, coins arrondis,
//! bordure, clic) avec un enfant optionnel.

use frus_core::{
    Border, BorderRadius, BoxDecoration, BoxShadow, Color, Insets, LinearGradient, Rect, Scene,
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
