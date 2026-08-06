//! Thème : *design tokens* (couleurs, rayon, espacement) injectés au rendu.
//!
//! Le thème est passé à [`crate::build_ui`] et transmis à `Widget::paint` ; les
//! widgets l'utilisent pour leurs valeurs par défaut (couleur de texte, champ de
//! saisie, barres de défilement…), sans empêcher une surcharge explicite.

use frus_core::{Color, FontWeight, TextDirection, TextStyle};

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

/// Les **rôles de couleur** (Material 3) — la **source de vérité** des couleurs
/// du thème. Les widgets référencent des rôles, jamais des couleurs littérales :
/// changer de schéma recolore toute l'app et garantit le contraste (paires
/// `X`/`on_X`). Écrit à la main clair/sombre ; `from_seed` (HCT) viendra après.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorScheme {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub secondary: Color,
    pub on_secondary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub background: Color,
    pub surface: Color,
    pub on_surface: Color,
    /// Surface tonale discrète (fonds de zones, pistes).
    pub surface_variant: Color,
    /// Contenu secondaire sur les surfaces (le `muted` historique).
    pub on_surface_variant: Color,
    /// Surface **élevée** (panneaux flottants, menus).
    pub surface_container: Color,
    /// Surface encore plus élevée (menus au-dessus de dialogues…).
    pub surface_container_high: Color,
    /// Surface inversée (toasts/snackbars qui tranchent sur le fond).
    pub inverse_surface: Color,
    pub on_inverse_surface: Color,
    /// Contours au repos.
    pub outline: Color,
    /// Contours discrets (séparateurs fins).
    pub outline_variant: Color,
    pub error: Color,
    pub on_error: Color,
    /// Voile des modales/tiroirs (l'alpha est appliqué à l'usage).
    pub scrim: Color,
    /// Couleur des ombres portées (l'alpha est appliqué à l'usage).
    pub shadow: Color,
}

impl ColorScheme {
    /// Schéma sombre.
    pub fn dark() -> Self {
        Self {
            primary: Color::rgb8(96, 200, 130),
            on_primary: Color::rgb8(16, 28, 20),
            primary_container: Color::rgb8(30, 64, 44),
            on_primary_container: Color::rgb8(178, 240, 200),
            secondary: Color::rgb8(150, 170, 200),
            on_secondary: Color::rgb8(20, 26, 36),
            secondary_container: Color::rgb8(44, 52, 68),
            on_secondary_container: Color::rgb8(205, 220, 240),
            background: Color::rgb8(18, 20, 24),
            surface: Color::rgb8(30, 33, 40),
            on_surface: Color::rgb8(230, 232, 236),
            surface_variant: Color::rgb8(38, 42, 52),
            on_surface_variant: Color::rgb8(150, 156, 168),
            surface_container: Color::rgb8(36, 40, 48),
            surface_container_high: Color::rgb8(44, 48, 58),
            inverse_surface: Color::rgb8(226, 228, 234),
            on_inverse_surface: Color::rgb8(28, 32, 38),
            outline: Color::rgb8(70, 76, 88),
            outline_variant: Color::rgb8(48, 52, 62),
            error: Color::rgb8(224, 108, 108),
            on_error: Color::rgb8(38, 12, 12),
            scrim: Color::BLACK,
            shadow: Color::BLACK,
        }
    }

    /// Schéma clair.
    pub fn light() -> Self {
        Self {
            primary: Color::rgb8(46, 160, 96),
            on_primary: Color::rgb8(255, 255, 255),
            primary_container: Color::rgb8(200, 238, 214),
            on_primary_container: Color::rgb8(10, 64, 36),
            secondary: Color::rgb8(90, 110, 150),
            on_secondary: Color::rgb8(255, 255, 255),
            secondary_container: Color::rgb8(220, 228, 244),
            on_secondary_container: Color::rgb8(30, 42, 66),
            background: Color::rgb8(245, 246, 248),
            surface: Color::rgb8(255, 255, 255),
            on_surface: Color::rgb8(28, 32, 38),
            surface_variant: Color::rgb8(238, 240, 244),
            on_surface_variant: Color::rgb8(110, 116, 126),
            surface_container: Color::rgb8(244, 245, 248),
            surface_container_high: Color::rgb8(238, 240, 244),
            inverse_surface: Color::rgb8(45, 50, 58),
            on_inverse_surface: Color::rgb8(240, 242, 246),
            outline: Color::rgb8(206, 210, 218),
            outline_variant: Color::rgb8(226, 230, 236),
            error: Color::rgb8(200, 64, 64),
            on_error: Color::rgb8(255, 255, 255),
            scrim: Color::BLACK,
            shadow: Color::BLACK,
        }
    }

    /// Génère un schéma complet depuis **une couleur graine** (Material 3
    /// « dynamic color », via [HCT](frus_core::Hct)). La teinte de la graine
    /// irrigue cinq palettes tonales (primaire, secondaire, tertiaire — non
    /// exposée pour l'instant —, neutres) ; chaque rôle est un **ton** précis
    /// de sa palette, ce qui garantit les contrastes des paires `X`/`on_X`.
    ///
    /// Écarts assumés vis-à-vis de la spec M3 : `surface` est légèrement
    /// décollée du `background` (nos cartes posent une surface sur le fond,
    /// tons 12/6 en sombre, 100/98 en clair) — la spec 2023 les confond.
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        use frus_core::{Hct, TonalPalette};

        let hct = Hct::from_color(seed);
        // Chromas M3 : la primaire garde le chroma de la graine (plancher 48),
        // les autres palettes sont des déclinaisons assourdies de la teinte.
        let primary = TonalPalette::new(hct.hue, hct.chroma.max(48.0));
        let secondary = TonalPalette::new(hct.hue, 16.0);
        let neutral = TonalPalette::new(hct.hue, 4.0);
        let neutral_variant = TonalPalette::new(hct.hue, 8.0);
        let error = TonalPalette::new(25.0, 84.0);

        let p = |tone: f64| primary.tone(tone);
        let s = |tone: f64| secondary.tone(tone);
        let n = |tone: f64| neutral.tone(tone);
        let nv = |tone: f64| neutral_variant.tone(tone);
        let e = |tone: f64| error.tone(tone);

        if dark {
            Self {
                primary: p(80.0),
                on_primary: p(20.0),
                primary_container: p(30.0),
                on_primary_container: p(90.0),
                secondary: s(80.0),
                on_secondary: s(20.0),
                secondary_container: s(30.0),
                on_secondary_container: s(90.0),
                background: n(6.0),
                surface: n(12.0),
                on_surface: n(90.0),
                surface_variant: nv(20.0),
                on_surface_variant: nv(80.0),
                surface_container: n(17.0),
                surface_container_high: n(22.0),
                inverse_surface: n(90.0),
                on_inverse_surface: n(20.0),
                outline: nv(60.0),
                outline_variant: nv(30.0),
                error: e(80.0),
                on_error: e(20.0),
                scrim: n(0.0),
                shadow: n(0.0),
            }
        } else {
            Self {
                primary: p(40.0),
                on_primary: p(100.0),
                primary_container: p(90.0),
                on_primary_container: p(10.0),
                secondary: s(40.0),
                on_secondary: s(100.0),
                secondary_container: s(90.0),
                on_secondary_container: s(10.0),
                background: n(98.0),
                surface: n(100.0),
                on_surface: n(10.0),
                surface_variant: nv(94.0),
                on_surface_variant: nv(30.0),
                surface_container: n(96.0),
                surface_container_high: n(94.0),
                inverse_surface: n(20.0),
                on_inverse_surface: n(95.0),
                outline: nv(50.0),
                outline_variant: nv(80.0),
                error: e(40.0),
                on_error: e(100.0),
                scrim: n(0.0),
                shadow: n(0.0),
            }
        }
    }

    /// Interpole rôle à rôle vers `other` (fondu de bascule clair/sombre).
    pub fn lerp(&self, other: &ColorScheme, t: f32) -> ColorScheme {
        let c = |a: Color, b: Color| a.lerp(b, t);
        ColorScheme {
            primary: c(self.primary, other.primary),
            on_primary: c(self.on_primary, other.on_primary),
            primary_container: c(self.primary_container, other.primary_container),
            on_primary_container: c(self.on_primary_container, other.on_primary_container),
            secondary: c(self.secondary, other.secondary),
            on_secondary: c(self.on_secondary, other.on_secondary),
            secondary_container: c(self.secondary_container, other.secondary_container),
            on_secondary_container: c(self.on_secondary_container, other.on_secondary_container),
            background: c(self.background, other.background),
            surface: c(self.surface, other.surface),
            on_surface: c(self.on_surface, other.on_surface),
            surface_variant: c(self.surface_variant, other.surface_variant),
            on_surface_variant: c(self.on_surface_variant, other.on_surface_variant),
            surface_container: c(self.surface_container, other.surface_container),
            surface_container_high: c(self.surface_container_high, other.surface_container_high),
            inverse_surface: c(self.inverse_surface, other.inverse_surface),
            on_inverse_surface: c(self.on_inverse_surface, other.on_inverse_surface),
            outline: c(self.outline, other.outline),
            outline_variant: c(self.outline_variant, other.outline_variant),
            error: c(self.error, other.error),
            on_error: c(self.on_error, other.on_error),
            scrim: c(self.scrim, other.scrim),
            shadow: c(self.shadow, other.shadow),
        }
    }
}

/// Ensemble de tokens de style.
///
/// La [`ColorScheme`] (`theme.scheme`) est la **source de vérité** des couleurs ;
/// les champs « à plat » (`background`, `surface`, `primary`, …) sont des **vues
/// de commodité** sur les rôles les plus employés, dérivées du schéma — l'API
/// historique des widgets reste intacte. `focus`/`selection` sont des accents
/// d'interaction propres à frus (hors rôles M3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Rôles de couleur (la source de vérité).
    pub scheme: ColorScheme,
    /// Fond de l'application (= `scheme.background`).
    pub background: Color,
    /// Fond des surfaces (= `scheme.surface`).
    pub surface: Color,
    /// Couleur d'accent (= `scheme.primary`).
    pub primary: Color,
    /// Texte/contenu sur `primary` (= `scheme.on_primary`).
    pub on_primary: Color,
    /// Texte par défaut sur les surfaces (= `scheme.on_surface`).
    pub on_surface: Color,
    /// Texte secondaire / éléments discrets (= `scheme.on_surface_variant`).
    pub muted: Color,
    /// Bordures au repos (= `scheme.outline`).
    pub border: Color,
    /// Accent de focus (accent d'interaction frus, hors schéma).
    pub focus: Color,
    /// Surbrillance de sélection de texte (idem).
    pub selection: Color,
    /// Conteneur d'accent tonal (= `scheme.primary_container`).
    pub primary_container: Color,
    /// Contenu sur `primary_container` (= `scheme.on_primary_container`).
    pub on_primary_container: Color,
    /// Couleur d'erreur / danger (= `scheme.error`).
    pub error: Color,
    /// Contenu sur `error` (= `scheme.on_error`).
    pub on_error: Color,
    /// Variante discrète de contour (= `scheme.outline_variant`).
    pub outline_variant: Color,
    /// Échelle typographique nommée (15 crans Material).
    pub text: TextTheme,
    /// Rayon de coin par défaut.
    pub radius: f32,
    /// Unité d'espacement de base.
    pub spacing: f32,
    /// **Direction de lecture/mise en page** ambiante (LTR par défaut). En RTL,
    /// le pilote retourne horizontalement la mise en page. Porté ici (contexte
    /// ambiant threadé jusqu'au paint) en attendant un `Env` dédié (§2).
    pub direction: TextDirection,
}

impl Theme {
    /// Construit un thème depuis un schéma : les champs plats sont **dérivés**
    /// des rôles (une seule source de vérité).
    pub fn from_scheme(scheme: ColorScheme, focus: Color, selection: Color) -> Self {
        Self {
            scheme,
            background: scheme.background,
            surface: scheme.surface,
            primary: scheme.primary,
            on_primary: scheme.on_primary,
            on_surface: scheme.on_surface,
            muted: scheme.on_surface_variant,
            border: scheme.outline,
            focus,
            selection,
            primary_container: scheme.primary_container,
            on_primary_container: scheme.on_primary_container,
            error: scheme.error,
            on_error: scheme.on_error,
            outline_variant: scheme.outline_variant,
            text: TextTheme::default(),
            radius: 10.0,
            spacing: 8.0,
            direction: TextDirection::Ltr,
        }
    }

    /// Le même thème en **droite-à-gauche** (arabe, hébreu…).
    pub fn rtl(mut self) -> Self {
        self.direction = TextDirection::Rtl;
        self
    }

    /// Thème sombre.
    pub fn dark() -> Self {
        Self::from_scheme(
            ColorScheme::dark(),
            Color::rgb8(90, 158, 242),
            Color::rgba(0.35, 0.62, 0.95, 0.40),
        )
    }

    /// Thème clair.
    pub fn light() -> Self {
        Self::from_scheme(
            ColorScheme::light(),
            Color::rgb8(40, 120, 220),
            Color::rgba(0.20, 0.50, 0.90, 0.30),
        )
    }

    /// Thème généré depuis une **couleur graine** (voir
    /// [`ColorScheme::from_seed`]). L'anneau de focus et la sélection dérivent
    /// de la primaire du schéma (rôles d'interaction propres à frus, hors
    /// schéma M3).
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        let scheme = ColorScheme::from_seed(seed, dark);
        let focus = scheme.primary;
        let selection = scheme.primary.with_alpha(if dark { 0.40 } else { 0.30 });
        Self::from_scheme(scheme, focus, selection)
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
    /// Sert au fondu de thème au basculement clair/sombre. Le **schéma** est
    /// interpolé rôle à rôle et les champs plats en sont re-dérivés (une seule
    /// source de vérité, même en cours de fondu).
    pub fn lerp(&self, other: &Theme, t: f32) -> Theme {
        let t = t.clamp(0.0, 1.0);
        let f = |a: f32, b: f32| a + (b - a) * t;
        let mut out = Theme::from_scheme(
            self.scheme.lerp(&other.scheme, t),
            self.focus.lerp(other.focus, t),
            self.selection.lerp(other.selection, t),
        );
        // La typographie ne participe pas au fondu (identique clair/sombre).
        out.text = self.text;
        out.radius = f(self.radius, other.radius);
        out.spacing = f(self.spacing, other.spacing);
        // La direction est discrète : on garde celle de la cible du fondu.
        out.direction = other.direction;
        out
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

    /// Contraste WCAG entre deux couleurs (rapport ≥ 1).
    fn contrast(a: Color, b: Color) -> f32 {
        let (la, lb) = (a.compute_luminance() + 0.05, b.compute_luminance() + 0.05);
        if la > lb {
            la / lb
        } else {
            lb / la
        }
    }

    #[test]
    fn from_seed_generates_contrasting_pairs() {
        // Toute paire X / on_X doit rester lisible (≥ 4,5:1, l'exigence AA),
        // pour n'importe quelle graine — même très peu chromatique.
        for seed in [
            Color::rgb8(0x42, 0x85, 0xF4), // bleu Google
            Color::rgb8(0x9C, 0x27, 0xB0), // violet
            Color::rgb8(0x80, 0x80, 0x80), // gris (chroma quasi nul)
        ] {
            for dark in [false, true] {
                let s = ColorScheme::from_seed(seed, dark);
                for (name, base, on) in [
                    ("primary", s.primary, s.on_primary),
                    ("secondary", s.secondary, s.on_secondary),
                    ("surface", s.surface, s.on_surface),
                    ("error", s.error, s.on_error),
                    ("inverse", s.inverse_surface, s.on_inverse_surface),
                ] {
                    let ratio = contrast(base, on);
                    assert!(
                        ratio >= 4.5,
                        "contraste {name} insuffisant ({ratio:.2}) — graine {seed:?}, dark={dark}"
                    );
                }
            }
        }
    }

    #[test]
    fn from_seed_light_and_dark_share_the_hue() {
        // Les deux modes déclinent la même teinte (la primaire sombre est la
        // version ton 80 de la primaire claire ton 40).
        let seed = Color::rgb8(0x42, 0x85, 0xF4);
        let light = ColorScheme::from_seed(seed, false);
        let dark = ColorScheme::from_seed(seed, true);
        let hue = |c: Color| frus_core::Hct::from_color(c).hue;
        let delta = (hue(light.primary) - hue(dark.primary)).abs();
        let delta = delta.min(360.0 - delta);
        assert!(
            delta < 12.0,
            "teintes clair/sombre divergentes ({delta:.1}°)"
        );
        // Et le sombre est bien… sombre.
        assert!(dark.background.compute_luminance() < 0.1);
        assert!(light.background.compute_luminance() > 0.85);
    }

    #[test]
    fn dark_and_light_differ() {
        assert_ne!(Theme::dark().background, Theme::light().background);
        assert_ne!(Theme::dark().on_surface, Theme::light().on_surface);
    }

    #[test]
    fn flat_fields_mirror_the_scheme() {
        // Les champs plats sont des vues dérivées du schéma — y compris au
        // milieu d'un fondu (le lerp passe par le schéma).
        for theme in [
            Theme::dark(),
            Theme::light(),
            Theme::dark().lerp(&Theme::light(), 0.37),
        ] {
            assert_eq!(theme.background, theme.scheme.background);
            assert_eq!(theme.surface, theme.scheme.surface);
            assert_eq!(theme.primary, theme.scheme.primary);
            assert_eq!(theme.on_primary, theme.scheme.on_primary);
            assert_eq!(theme.on_surface, theme.scheme.on_surface);
            assert_eq!(theme.muted, theme.scheme.on_surface_variant);
            assert_eq!(theme.border, theme.scheme.outline);
            assert_eq!(theme.primary_container, theme.scheme.primary_container);
            assert_eq!(theme.error, theme.scheme.error);
            assert_eq!(theme.outline_variant, theme.scheme.outline_variant);
        }
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
