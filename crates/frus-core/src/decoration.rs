//! Modèle de **décoration de boîte** : le vocabulaire de peinture d'un rectangle
//! (fond, dégradé, bordure, coins arrondis, ombre), indépendant du widget et du
//! thème.
//!
//! Une [`BoxDecoration`] est une valeur pure `Copy` qu'un widget compose au paint,
//! puis **abaisse** en primitives de [`Scene`] via [`BoxDecoration::paint_into`],
//! dans un **ordre fixe** : ombre → fond (couleur/dégradé) → bordure. Elle alimente
//! aussi la mise en page : [`BoxDecoration::content_padding`] réserve la place de la
//! bordure pour taffy.

use crate::{Color, Insets, Rect, Scene};

/// Rayons d'arrondi **par coin** (px logiques). `From<f32>` fournit le cas
/// uniforme : partout où un rayon est attendu, un simple `10.0` reste valide.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    /// Aucun arrondi.
    pub const ZERO: Self = Self::uniform(0.0);

    /// Le même rayon aux quatre coins.
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Seuls les coins **hauts** arrondis (en-têtes, feuilles montantes).
    pub const fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }

    /// Seuls les coins **bas** arrondis.
    pub const fn bottom(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Rayons **bornés à zéro** (un rayon négatif n'a pas de sens au rendu).
    pub fn clamped(self) -> Self {
        Self {
            top_left: self.top_left.max(0.0),
            top_right: self.top_right.max(0.0),
            bottom_right: self.bottom_right.max(0.0),
            bottom_left: self.bottom_left.max(0.0),
        }
    }

    /// Chaque coin augmenté de `by` (l'enveloppe d'une ombre floutée).
    pub fn inflate(self, by: f32) -> Self {
        Self {
            top_left: self.top_left + by,
            top_right: self.top_right + by,
            bottom_right: self.bottom_right + by,
            bottom_left: self.bottom_left + by,
        }
    }

    /// Tous les rayons multipliés par `factor` (échelle DPI).
    pub fn scale(self, factor: f32) -> Self {
        Self {
            top_left: self.top_left * factor,
            top_right: self.top_right * factor,
            bottom_right: self.bottom_right * factor,
            bottom_left: self.bottom_left * factor,
        }
    }

    /// `[tl, tr, br, bl]`, prêt pour le GPU.
    pub fn to_array(self) -> [f32; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
}

impl From<f32> for BorderRadius {
    fn from(radius: f32) -> Self {
        Self::uniform(radius)
    }
}

/// Une bordure uniforme (même épaisseur/couleur sur les quatre côtés).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    /// Épaisseur, en pixels logiques.
    pub width: f32,
    /// Couleur du trait.
    pub color: Color,
}

impl Border {
    /// Une bordure uniforme.
    pub const fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }

    /// `true` si la bordure est visible (épaisseur et alpha non nuls).
    pub fn is_visible(&self) -> bool {
        self.width > 0.0 && self.color.a > 0.0
    }
}

/// Un dégradé **linéaire** : du fond (`BoxDecoration::color`) vers `end`, le long
/// de `direction` exprimée en espace `[0,1]²` (`[0,1]` = haut→bas, `[1,0]` =
/// gauche→droite).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearGradient {
    /// Couleur d'arrivée (la couleur de départ est le fond de la décoration).
    pub end: Color,
    /// Direction du dégradé en espace `[0,1]²`.
    pub direction: [f32; 2],
}

impl LinearGradient {
    /// Un dégradé linéaire vers `end`, dans la direction donnée.
    pub const fn new(end: Color, direction: [f32; 2]) -> Self {
        Self { end, direction }
    }
}

/// Une ombre portée douce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    /// Couleur (l'alpha règle l'intensité).
    pub color: Color,
    /// Décalage `(dx, dy)`, en pixels logiques.
    pub offset: (f32, f32),
    /// Rayon de flou.
    pub blur: f32,
    /// Élargissement de l'ombre au-delà de la boîte (avant flou).
    pub spread: f32,
}

impl BoxShadow {
    /// Une ombre `(dx, dy)` de flou `blur`, sans élargissement.
    pub const fn new(dx: f32, dy: f32, blur: f32, color: Color) -> Self {
        Self {
            color,
            offset: (dx, dy),
            blur,
            spread: 0.0,
        }
    }

    /// Fixe l'élargissement (`spread`).
    pub const fn spread(mut self, spread: f32) -> Self {
        self.spread = spread;
        self
    }

    /// Rectangle occupé par l'ombre autour de `rect` (décalage + flou + spread).
    pub fn bounds(&self, rect: Rect) -> Rect {
        let grow = self.blur + self.spread;
        Rect::new(
            rect.x + self.offset.0 - grow,
            rect.y + self.offset.1 - grow,
            rect.width + 2.0 * grow,
            rect.height + 2.0 * grow,
        )
    }
}

/// La décoration complète d'une boîte rectangulaire.
///
/// Ordre de peinture **fixe** (comme Flutter) : ombre → fond → bordure. Le fond est
/// soit uni (`color`), soit dégradé (`color` → `gradient.end`). Une bordure sans
/// fond peint un contour sur fond transparent ; une décoration entièrement vide ne
/// peint rien.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxDecoration {
    /// Couleur de fond (aussi couleur de départ d'un éventuel dégradé).
    pub color: Option<Color>,
    /// Dégradé linéaire du fond.
    pub gradient: Option<LinearGradient>,
    /// Bordure uniforme.
    pub border: Option<Border>,
    /// Rayons des coins arrondis, par coin.
    pub radius: BorderRadius,
    /// Ombre portée.
    pub shadow: Option<BoxShadow>,
}

impl BoxDecoration {
    /// Une décoration à fond uni.
    pub fn filled(color: Color) -> Self {
        Self {
            color: Some(color),
            ..Default::default()
        }
    }

    /// Fixe les rayons des coins (uniforme via `f32`, par coin via
    /// [`BorderRadius`]).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Ajoute une bordure uniforme.
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    /// Ajoute une ombre.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Ajoute un dégradé linéaire du fond.
    pub fn gradient(mut self, gradient: LinearGradient) -> Self {
        self.gradient = Some(gradient);
        self
    }

    /// Marge intérieure réservée à la bordure — à ajouter au padding pour que le
    /// contenu ne soit pas mangé par le trait (alimente taffy).
    pub fn content_padding(&self) -> Insets {
        match self.border {
            Some(b) if b.is_visible() => Insets::uniform(b.width),
            _ => Insets::ZERO,
        }
    }

    /// Abaisse la décoration en primitives de `scene`, dans l'ordre fixe
    /// ombre → fond → bordure. `opacity` (`0..=1`) module **toutes** les couleurs
    /// (fondu d'apparition). `rect` est la boîte en coordonnées absolues.
    pub fn paint_into(&self, scene: &mut Scene, rect: Rect, opacity: f32) {
        // 1) Ombre, derrière le reste.
        if let Some(shadow) = self.shadow {
            scene.shadow(
                shadow.bounds(rect),
                shadow.color.fade(opacity),
                self.radius.inflate(shadow.blur + shadow.spread),
                shadow.blur,
            );
        }

        // 2/3) Fond (uni ou dégradé) + bordure, en une primitive.
        let (border_width, border_color) = match self.border {
            Some(b) => (b.width, b.color.fade(opacity)),
            None => (0.0, Color::TRANSPARENT),
        };
        let has_border = self.border.map(|b| b.is_visible()).unwrap_or(false);

        match (self.color, self.gradient) {
            (Some(color), Some(gradient)) => scene.gradient_rect(
                rect,
                color.fade(opacity),
                gradient.end.fade(opacity),
                gradient.direction,
                self.radius,
                border_width,
                border_color,
            ),
            (Some(color), None) => scene.draw_rect(
                rect,
                color.fade(opacity),
                self.radius,
                border_width,
                border_color,
            ),
            // Bordure seule (sans fond) : contour sur fond transparent.
            (None, _) if has_border => scene.draw_rect(
                rect,
                Color::TRANSPARENT,
                self.radius,
                border_width,
                border_color,
            ),
            // Rien à peindre.
            (None, _) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Primitive;

    fn rect() -> Rect {
        Rect::new(10.0, 20.0, 100.0, 40.0)
    }

    #[test]
    fn content_padding_reserves_the_border() {
        let deco = BoxDecoration::filled(Color::WHITE).border(Border::new(2.0, Color::BLACK));
        assert_eq!(deco.content_padding(), Insets::uniform(2.0));
        // Bordure invisible (largeur nulle) → pas de padding.
        let none = BoxDecoration::filled(Color::WHITE).border(Border::new(0.0, Color::BLACK));
        assert_eq!(none.content_padding(), Insets::ZERO);
    }

    #[test]
    fn empty_decoration_paints_nothing() {
        let mut scene = Scene::new();
        BoxDecoration::default().paint_into(&mut scene, rect(), 1.0);
        assert!(scene.is_empty());
    }

    #[test]
    fn fixed_paint_order_shadow_then_fill() {
        let mut scene = Scene::new();
        let deco = BoxDecoration::filled(Color::WHITE)
            .radius(6.0)
            .shadow(BoxShadow::new(
                0.0,
                4.0,
                8.0,
                Color::rgba(0.0, 0.0, 0.0, 0.5),
            ));
        deco.paint_into(&mut scene, rect(), 1.0);
        // Deux primitives : l'ombre d'abord, puis le fond.
        assert_eq!(scene.len(), 2);
        match scene.primitives()[0] {
            Primitive::Rect { blur, .. } => assert!(blur > 0.0, "1re primitive = ombre floue"),
            _ => panic!("attendu un rectangle"),
        }
        match scene.primitives()[1] {
            Primitive::Rect { blur, color, .. } => {
                assert_eq!(blur, 0.0, "2e primitive = fond net");
                assert_eq!(color, Color::WHITE);
            }
            _ => panic!("attendu un rectangle"),
        }
    }

    #[test]
    fn opacity_fades_all_colours() {
        let mut scene = Scene::new();
        BoxDecoration::filled(Color::rgb(1.0, 0.0, 0.0))
            .border(Border::new(2.0, Color::rgb(0.0, 1.0, 0.0)))
            .paint_into(&mut scene, rect(), 0.5);
        match scene.primitives()[0] {
            Primitive::Rect {
                color,
                border_color,
                ..
            } => {
                assert_eq!(color.a, 0.5);
                assert_eq!(border_color.a, 0.5);
            }
            _ => panic!("attendu un rectangle"),
        }
    }

    #[test]
    fn border_only_paints_transparent_fill_with_stroke() {
        let mut scene = Scene::new();
        BoxDecoration::default()
            .border(Border::new(1.0, Color::WHITE))
            .paint_into(&mut scene, rect(), 1.0);
        assert_eq!(scene.len(), 1);
        match scene.primitives()[0] {
            Primitive::Rect {
                color,
                border_width,
                ..
            } => {
                assert_eq!(color, Color::TRANSPARENT);
                assert_eq!(border_width, 1.0);
            }
            _ => panic!("attendu un rectangle"),
        }
    }

    #[test]
    fn shadow_bounds_grow_with_blur_and_spread() {
        let s = BoxShadow::new(0.0, 0.0, 4.0, Color::BLACK).spread(2.0);
        let b = s.bounds(Rect::new(0.0, 0.0, 10.0, 10.0));
        // grow = blur + spread = 6 de chaque côté.
        assert_eq!(b, Rect::new(-6.0, -6.0, 22.0, 22.0));
    }
}
