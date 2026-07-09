//! [`Container`] : une boîte décorée (taille, marge, couleur, coins arrondis,
//! bordure, clic) avec un enfant optionnel.

use frus_core::{Color, Insets, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::{Interaction, Status};
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
    radius: f32,
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
            radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            color: None,
            hover_color: None,
            pressed_color: None,
            gradient: None,
            shadow: None,
            on_click: None,
            children: Vec::new(),
        }
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

    /// Rayon des coins arrondis, en pixels.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
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
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            padding: self.padding,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, scene: &mut Scene) {
        // Pressé : instantané. Sinon, transition animée repos → survol.
        let color = if status.interaction == Interaction::Pressed {
            self.pressed_color.or(self.hover_color).or(self.color)
        } else if let (Some(base), Some(hover)) = (self.color, self.hover_color) {
            Some(base.lerp(hover, ease(status.hover_progress)))
        } else {
            self.color
        };

        // Ombre portée (dessinée derrière, légèrement plus grande et floue).
        if let Some((dx, dy, blur, shadow_color)) = self.shadow {
            let shadow_rect = Rect::new(
                bounds.x + dx - blur,
                bounds.y + dy - blur,
                bounds.width + 2.0 * blur,
                bounds.height + 2.0 * blur,
            );
            scene.shadow(shadow_rect, shadow_color, self.radius + blur, blur);
        }

        // Fond : dégradé si défini, sinon uni ; bordure éventuelle.
        let has_border = self.border_width > 0.0 && self.border_color.a > 0.0;
        match (color, self.gradient) {
            (Some(color), Some((end, dir))) => scene.gradient_rect(
                bounds,
                color,
                end,
                dir,
                self.radius,
                self.border_width,
                self.border_color,
            ),
            (Some(color), None) => {
                scene.draw_rect(bounds, color, self.radius, self.border_width, self.border_color)
            }
            (None, _) if has_border => scene.draw_rect(
                bounds,
                Color::TRANSPARENT,
                self.radius,
                self.border_width,
                self.border_color,
            ),
            (None, _) => {}
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.on_click.clone()
    }
}
