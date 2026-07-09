//! La [`Scene`] : une liste d'affichage pure, indépendante du backend de rendu.
//!
//! Elle décrit *ce qu'il faut dessiner* (des primitives) sans rien savoir du
//! GPU. `frus-gpu` la consomme pour produire des commandes GPU ; `frus-widgets`
//! la produit à partir d'un arbre de widgets.
//!
//! Chaque primitive porte un **rectangle de découpe** (`clip`) ; on le fixe via
//! [`Scene::set_clip`] avant d'ajouter des primitives.

use crate::{Color, Point, Rect};

/// Une primitive de dessin.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    /// Un rectangle, éventuellement à coins arrondis et bordé.
    Rect {
        rect: Rect,
        color: Color,
        radius: f32,
        border_width: f32,
        border_color: Color,
        /// Rectangle de découpe : rien n'est dessiné en dehors.
        clip: Rect,
    },
    /// Une ligne de texte, ancrée par son coin haut-gauche.
    Text {
        position: Point,
        text: String,
        size: f32,
        color: Color,
        /// Rectangle de découpe.
        clip: Rect,
    },
}

/// Une scène 2D : la description déclarative de ce qu'il faut dessiner.
#[derive(Clone, Debug)]
pub struct Scene {
    primitives: Vec<Primitive>,
    current_clip: Rect,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            primitives: Vec::new(),
            current_clip: Rect::UNBOUNDED,
        }
    }
}

impl Scene {
    /// Crée une scène vide (découpe neutre).
    pub fn new() -> Self {
        Self::default()
    }

    /// Vide la scène pour la réutiliser à la frame suivante.
    pub fn clear(&mut self) {
        self.primitives.clear();
        self.current_clip = Rect::UNBOUNDED;
    }

    /// Fixe le rectangle de découpe appliqué aux primitives suivantes.
    pub fn set_clip(&mut self, clip: Rect) {
        self.current_clip = clip;
    }

    /// Ajoute un rectangle plein (coins droits, sans bordure).
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            clip: self.current_clip,
        });
    }

    /// Ajoute un rectangle avec coins arrondis et/ou bordure.
    pub fn draw_rect(
        &mut self,
        rect: Rect,
        color: Color,
        radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            radius,
            border_width,
            border_color,
            clip: self.current_clip,
        });
    }

    /// Ajoute une ligne de texte, ancrée par son coin haut-gauche.
    pub fn text(&mut self, position: Point, text: impl Into<String>, size: f32, color: Color) {
        self.primitives.push(Primitive::Text {
            position,
            text: text.into(),
            size,
            color,
            clip: self.current_clip,
        });
    }

    /// Nombre de primitives dans la scène.
    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    /// `true` si la scène ne contient aucune primitive.
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    /// Les primitives, dans l'ordre d'insertion (= ordre de dessin).
    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Rect};

    #[test]
    fn fill_rect_pushes_primitive_with_current_clip() {
        let mut scene = Scene::new();
        assert!(scene.is_empty());
        scene.fill_rect(Rect::new(1.0, 2.0, 3.0, 4.0), Color::WHITE);
        assert_eq!(scene.len(), 1);
        assert_eq!(
            scene.primitives()[0],
            Primitive::Rect {
                rect: Rect::new(1.0, 2.0, 3.0, 4.0),
                color: Color::WHITE,
                radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                clip: Rect::UNBOUNDED,
            }
        );
    }

    #[test]
    fn set_clip_applies_to_following_primitives() {
        let mut scene = Scene::new();
        let clip = Rect::new(0.0, 0.0, 10.0, 10.0);
        scene.set_clip(clip);
        scene.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::BLACK);
        if let Primitive::Rect { clip: c, .. } = scene.primitives()[0] {
            assert_eq!(c, clip);
        } else {
            panic!("attendu un rectangle");
        }
    }
}
