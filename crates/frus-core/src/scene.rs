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
    /// Un rectangle : coins arrondis, bordure, dégradé et/ou ombre douce.
    Rect {
        rect: Rect,
        /// Couleur de remplissage (couleur de départ si dégradé).
        color: Color,
        /// Couleur de fin du dégradé (`== color` si uni).
        color2: Color,
        /// Direction du dégradé en espace `[0,1]²` ; `(0,0)` = uni.
        gradient_dir: [f32; 2],
        radius: f32,
        border_width: f32,
        border_color: Color,
        /// Adoucissement du bord, en pixels (0 = net ; > 0 = ombre floue).
        blur: f32,
        /// Rectangle de découpe : rien n'est dessiné en dehors.
        clip: Rect,
        /// Identité du widget émetteur (pour l'animation de sortie).
        owner: u64,
    },
    /// Une ligne de texte, ancrée par son coin haut-gauche.
    Text {
        position: Point,
        text: String,
        size: f32,
        color: Color,
        /// Rectangle de découpe.
        clip: Rect,
        /// Identité du widget émetteur.
        owner: u64,
    },
}

impl Primitive {
    /// Identité du widget qui a émis cette primitive.
    pub fn owner(&self) -> u64 {
        match self {
            Primitive::Rect { owner, .. } => *owner,
            Primitive::Text { owner, .. } => *owner,
        }
    }
}

/// Une scène 2D : la description déclarative de ce qu'il faut dessiner.
#[derive(Clone, Debug)]
pub struct Scene {
    primitives: Vec<Primitive>,
    current_clip: Rect,
    current_owner: u64,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            primitives: Vec::new(),
            current_clip: Rect::UNBOUNDED,
            current_owner: 0,
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
        self.current_owner = 0;
    }

    /// Fixe le rectangle de découpe appliqué aux primitives suivantes.
    pub fn set_clip(&mut self, clip: Rect) {
        self.current_clip = clip;
    }

    /// Fixe l'identité du widget émetteur des primitives suivantes.
    pub fn set_owner(&mut self, owner: u64) {
        self.current_owner = owner;
    }

    /// Rejoue une primitive existante avec une opacité réduite (fondu de sortie).
    pub fn push_faded(&mut self, primitive: &Primitive, opacity: f32) {
        let faded = match primitive.clone() {
            Primitive::Rect {
                rect,
                color,
                color2,
                gradient_dir,
                radius,
                border_width,
                border_color,
                blur,
                clip,
                owner,
            } => Primitive::Rect {
                rect,
                color: color.fade(opacity),
                color2: color2.fade(opacity),
                gradient_dir,
                radius,
                border_width,
                border_color: border_color.fade(opacity),
                blur,
                clip,
                owner,
            },
            Primitive::Text {
                position,
                text,
                size,
                color,
                clip,
                owner,
            } => Primitive::Text {
                position,
                text,
                size,
                color: color.fade(opacity),
                clip,
                owner,
            },
        };
        self.primitives.push(faded);
    }

    /// Ajoute un rectangle plein (coins droits, sans bordure).
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
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
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius,
            border_width,
            border_color,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute un rectangle à dégradé linéaire (`color` → `color2` selon `dir`).
    pub fn gradient_rect(
        &mut self,
        rect: Rect,
        color: Color,
        color2: Color,
        dir: [f32; 2],
        radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2,
            gradient_dir: dir,
            radius,
            border_width,
            border_color,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute une ombre douce (rectangle arrondi au bord flou), sans bordure.
    pub fn shadow(&mut self, rect: Rect, color: Color, radius: f32, blur: f32) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            blur,
            clip: self.current_clip,
            owner: self.current_owner,
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
            owner: self.current_owner,
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
                color2: Color::WHITE,
                gradient_dir: [0.0, 0.0],
                radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                blur: 0.0,
                clip: Rect::UNBOUNDED,
                owner: 0,
            }
        );
    }

    #[test]
    fn push_faded_scales_alpha_and_keeps_owner() {
        let mut scene = Scene::new();
        scene.set_owner(42);
        scene.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::rgba(1.0, 0.0, 0.0, 1.0));
        let source = scene.primitives()[0].clone();
        assert_eq!(source.owner(), 42);

        let mut target = Scene::new();
        target.push_faded(&source, 0.5);
        if let Primitive::Rect { color, owner, .. } = target.primitives()[0] {
            assert_eq!(color.a, 0.5);
            assert_eq!(owner, 42);
        } else {
            panic!("attendu un rectangle");
        }
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
