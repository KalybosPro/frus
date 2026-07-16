//! Thème : *design tokens* (couleurs, rayon, espacement) injectés au rendu.
//!
//! Le thème est passé à [`crate::build_ui`] et transmis à `Widget::paint` ; les
//! widgets l'utilisent pour leurs valeurs par défaut (couleur de texte, champ de
//! saisie, barres de défilement…), sans empêcher une surcharge explicite.

use frus_core::{Color, FontWeight, TextStyle};

use crate::interaction::{Interaction, Status};

/// Échelle typographique **nommée** (les 15 crans de Material 3). Les widgets
/// choisissent un cran (`theme.text.title_medium`), jamais une taille en dur —
/// changer l'échelle retypographie toute l'app. Les couleurs restent héritées
/// (`None` → résolues contre le thème au paint).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTheme {
    pub display_large: TextStyle,
    pub display_medium: TextStyle,
    pub display_small: TextStyle,
    pub headline_large: TextStyle,
    pub headline_medium: TextStyle,
    pub headline_small: TextStyle,
    pub title_large: TextStyle,
    pub title_medium: TextStyle,
    pub title_small: TextStyle,
    pub body_large: TextStyle,
    pub body_medium: TextStyle,
    pub body_small: TextStyle,
    pub label_large: TextStyle,
    pub label_medium: TextStyle,
    pub label_small: TextStyle,
}

impl Default for TextTheme {
    /// L'échelle Material 3 de référence (tailles en px logiques ; les crans
    /// title/label portent une graisse medium, comme le spec).
    fn default() -> Self {
        let medium = |size: f32| TextStyle::new(size).weight(FontWeight::Medium);
        Self {
            display_large: TextStyle::new(57.0),
            display_medium: TextStyle::new(45.0),
            display_small: TextStyle::new(36.0),
            headline_large: TextStyle::new(32.0),
            headline_medium: TextStyle::new(28.0),
            headline_small: TextStyle::new(24.0),
            title_large: TextStyle::new(22.0),
            title_medium: medium(16.0),
            title_small: medium(14.0),
            body_large: TextStyle::new(16.0),
            body_medium: TextStyle::new(14.0),
            body_small: TextStyle::new(12.0),
            label_large: medium(14.0),
            label_medium: medium(12.0),
            label_small: medium(11.0),
        }
    }
}

/// Ensemble de tokens de style.
///
/// Les champs « à plat » (`background`, `surface`, `primary`, …) sont les **rôles
/// sémantiques** de base — les widgets s'y réfèrent, jamais à des couleurs
/// littérales. Un jeu de rôles Material étendu (conteneurs, erreur, contour
/// discret) complète le tout ; l'objectif à terme est une `ColorScheme` dérivée
/// d'une graine, mais l'écriture manuelle clair/sombre vient d'abord.
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
    /// Texte secondaire / éléments discrets (≈ `on_surface_variant` de M3).
    pub muted: Color,
    /// Bordures au repos (≈ `outline` de M3).
    pub border: Color,
    /// Accent de focus.
    pub focus: Color,
    /// Surbrillance de sélection de texte.
    pub selection: Color,
    /// Conteneur d'accent tonal (puces douces, surfaces sélectionnées légères).
    pub primary_container: Color,
    /// Contenu sur `primary_container`.
    pub on_primary_container: Color,
    /// Couleur d'erreur / danger.
    pub error: Color,
    /// Contenu sur `error`.
    pub on_error: Color,
    /// Variante discrète de contour (séparateurs fins, traits internes).
    pub outline_variant: Color,
    /// Échelle typographique nommée (15 crans Material).
    pub text: TextTheme,
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
            primary_container: Color::rgb8(30, 64, 44),
            on_primary_container: Color::rgb8(178, 240, 200),
            error: Color::rgb8(224, 108, 108),
            on_error: Color::rgb8(38, 12, 12),
            outline_variant: Color::rgb8(48, 52, 62),
            text: TextTheme::default(),
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
            primary_container: Color::rgb8(200, 238, 214),
            on_primary_container: Color::rgb8(10, 64, 36),
            error: Color::rgb8(200, 64, 64),
            on_error: Color::rgb8(255, 255, 255),
            outline_variant: Color::rgb8(226, 230, 236),
            text: TextTheme::default(),
            radius: 10.0,
            spacing: 8.0,
        }
    }

    /// Applique la **state-layer** Material sur `base` : superpose la couleur de
    /// contenu `on` à faible opacité selon l'état d'interaction — survol 8 %,
    /// focus 10 %, pression 12 % — en tenant compte des progressions animées
    /// (`hover_progress`/`focus_progress`). C'est la règle d'états **bakée** dans le
    /// thème : les widgets restent déclaratifs (ils passent leur couleur de base et
    /// leur couleur de contenu, le thème décide de l'overlay).
    pub fn state_layer(&self, base: Color, on: Color, status: &Status) -> Color {
        let mut overlay = 0.08 * status.hover_progress.clamp(0.0, 1.0)
            + 0.10 * status.focus_progress.clamp(0.0, 1.0);
        if status.interaction == Interaction::Pressed {
            overlay += 0.12;
        }
        base.lerp(on, overlay.min(1.0))
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
            primary_container: self.primary_container.lerp(other.primary_container, t),
            on_primary_container: self.on_primary_container.lerp(other.on_primary_container, t),
            error: self.error.lerp(other.error, t),
            on_error: self.on_error.lerp(other.on_error, t),
            outline_variant: self.outline_variant.lerp(other.outline_variant, t),
            // La typographie ne participe pas au fondu (identique clair/sombre).
            text: self.text,
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
    fn state_layer_darkens_toward_content_on_interaction() {
        let theme = Theme::dark();
        let base = Color::rgb(0.4, 0.4, 0.4);
        let on = Color::BLACK;

        // Au repos : aucune superposition.
        let idle = Status::default();
        assert_eq!(theme.state_layer(base, on, &idle), base);

        // Survolé à fond : base tirée de 8 % vers `on` (plus sombre ici).
        let hovered = Status {
            hover_progress: 1.0,
            ..Default::default()
        };
        let h = theme.state_layer(base, on, &hovered);
        assert!(h.r < base.r && (base.r - h.r - 0.4 * 0.08).abs() < 1e-4);

        // Pressé : superposition plus forte que le survol seul.
        let pressed = Status {
            interaction: Interaction::Pressed,
            ..Default::default()
        };
        assert!(theme.state_layer(base, on, &pressed).r < h.r);
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
