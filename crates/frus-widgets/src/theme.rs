//! Thème : *design tokens* (couleurs, rayon, espacement) injectés au rendu.
//!
//! Le thème est passé à [`crate::build_ui`] et transmis à `Widget::paint` ; les
//! widgets l'utilisent pour leurs valeurs par défaut (couleur de texte, champ de
//! saisie, barres de défilement…), sans empêcher une surcharge explicite.

use frus_core::Color;

/// Ensemble de tokens de style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Fond de l'application.
    pub background: Color,
    /// Fond des surfaces (cartes, champs, panneaux).
    pub surface: Color,
    /// Couleur d'accent (boutons principaux).
    pub primary: Color,
    /// Texte/contenu sur `primary`.
    pub on_primary: Color,
    /// Texte par défaut sur les surfaces.
    pub on_surface: Color,
    /// Texte secondaire / éléments discrets.
    pub muted: Color,
    /// Bordures au repos.
    pub border: Color,
    /// Accent de focus.
    pub focus: Color,
    /// Surbrillance de sélection de texte.
    pub selection: Color,
    /// Rayon de coin par défaut.
    pub radius: f32,
    /// Unité d'espacement de base.
    pub spacing: f32,
}

impl Theme {
    /// Thème sombre.
    pub fn dark() -> Self {
        Self {
            background: Color::rgb8(18, 20, 24),
            surface: Color::rgb8(30, 33, 40),
            primary: Color::rgb8(96, 200, 130),
            on_primary: Color::rgb8(16, 28, 20),
            on_surface: Color::rgb8(230, 232, 236),
            muted: Color::rgb8(150, 156, 168),
            border: Color::rgb8(70, 76, 88),
            focus: Color::rgb8(90, 158, 242),
            selection: Color::rgba(0.35, 0.62, 0.95, 0.40),
            radius: 10.0,
            spacing: 8.0,
        }
    }

    /// Thème clair.
    pub fn light() -> Self {
        Self {
            background: Color::rgb8(245, 246, 248),
            surface: Color::rgb8(255, 255, 255),
            primary: Color::rgb8(46, 160, 96),
            on_primary: Color::rgb8(255, 255, 255),
            on_surface: Color::rgb8(28, 32, 38),
            muted: Color::rgb8(110, 116, 126),
            border: Color::rgb8(206, 210, 218),
            focus: Color::rgb8(40, 120, 220),
            selection: Color::rgba(0.20, 0.50, 0.90, 0.30),
            radius: 10.0,
            spacing: 8.0,
        }
    }
}

impl Theme {
    /// Interpole vers `other` à l'avancement `t` (`0` = `self`, `1` = `other`).
    /// Sert au fondu de thème au basculement clair/sombre.
    pub fn lerp(&self, other: &Theme, t: f32) -> Theme {
        let t = t.clamp(0.0, 1.0);
        let f = |a: f32, b: f32| a + (b - a) * t;
        Theme {
            background: self.background.lerp(other.background, t),
            surface: self.surface.lerp(other.surface, t),
            primary: self.primary.lerp(other.primary, t),
            on_primary: self.on_primary.lerp(other.on_primary, t),
            on_surface: self.on_surface.lerp(other.on_surface, t),
            muted: self.muted.lerp(other.muted, t),
            border: self.border.lerp(other.border, t),
            focus: self.focus.lerp(other.focus, t),
            selection: self.selection.lerp(other.selection, t),
            radius: f(self.radius, other.radius),
            spacing: f(self.spacing, other.spacing),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_differ() {
        assert_ne!(Theme::dark().background, Theme::light().background);
        assert_ne!(Theme::dark().on_surface, Theme::light().on_surface);
    }

    #[test]
    fn lerp_hits_endpoints() {
        let d = Theme::dark();
        let l = Theme::light();
        assert_eq!(d.lerp(&l, 0.0).background, d.background);
        assert_eq!(d.lerp(&l, 1.0).background, l.background);
        // Au milieu : ni l'un ni l'autre.
        let mid = d.lerp(&l, 0.5).background;
        assert_ne!(mid, d.background);
        assert_ne!(mid, l.background);
    }
}
