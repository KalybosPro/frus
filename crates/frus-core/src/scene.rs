//! La [`Scene`] : une liste d'affichage pure, indépendante du backend de rendu.
//!
//! Elle décrit *ce qu'il faut dessiner* (des primitives) sans rien savoir du
//! GPU. `frus-gpu` la consomme pour produire des commandes GPU ; `frus-widgets`
//! la produit à partir d'un arbre de widgets.
//!
//! Chaque primitive porte un **rectangle de découpe** (`clip`) ; on le fixe via
//! [`Scene::set_clip`] avant d'ajouter des primitives.

use crate::{
    BorderRadius, Color, FontWeight, ImageHandle, Path, Point, Rect, Stroke, TextDecoration,
    TextRun, TextStyle,
};

/// Transformation affine appliquée à un **calque** ([`Primitive::Layer`]) au
/// moment du compositing : une **rotation** d'`angle` (radians, sens horaire)
/// autour de `pivot` (px écran). Le calque est d'abord rendu **à plat** dans une
/// texture (comme pour l'opacité de groupe), puis composité **tourné** — une seule
/// passe gère ainsi la rotation de tout un sous-arbre (rects, texte, images…), sans
/// toucher les shaders de chaque primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerTransform {
    /// Angle de rotation, en radians (sens horaire, y vers le bas).
    pub angle: f32,
    /// Centre de rotation, en pixels écran.
    pub pivot: Point,
}

impl LayerTransform {
    /// Une rotation d'`angle` radians autour de `pivot`.
    pub const fn rotation(angle: f32, pivot: Point) -> Self {
        Self { angle, pivot }
    }

    /// Met à l'échelle le pivot (l'angle est invariant par mise à l'échelle
    /// uniforme) — pour suivre `Primitive::scaled`.
    pub fn scaled(self, factor: f32) -> Self {
        Self {
            angle: self.angle,
            pivot: self.pivot.scale(factor),
        }
    }

    /// Décale le pivot — pour suivre `Primitive::translated`.
    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self {
            angle: self.angle,
            pivot: Point::new(self.pivot.x + dx, self.pivot.y + dy),
        }
    }
}

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
        /// Rayons d'arrondi, par coin.
        radius: BorderRadius,
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
        /// Graisse de police.
        weight: FontWeight,
        /// Italique.
        italic: bool,
        /// Largeur de repli : au-delà, le texte revient à la ligne (`None` =
        /// pas de repli — les `\n` explicites font les lignes).
        max_width: Option<f32>,
        /// Lignes de décoration (soulignement, barré…).
        decoration: TextDecoration,
        /// Couleur des décorations ; `None` = la couleur du texte.
        decoration_color: Option<Color>,
        /// Rectangle de découpe.
        clip: Rect,
        /// Identité du widget émetteur.
        owner: u64,
    },
    /// Du texte **riche** : une suite de runs résolus (styles/couleurs mêlés),
    /// mise en forme d'un seul tenant (une seule ligne de base partagée).
    RichText {
        position: Point,
        runs: Vec<TextRun>,
        /// Largeur de repli : au-delà, le texte revient à la ligne (`None` =
        /// pas de repli).
        max_width: Option<f32>,
        /// Rectangle de découpe.
        clip: Rect,
        /// Identité du widget émetteur.
        owner: u64,
    },
    /// Un **chemin vectoriel** : géométrie 2D arbitraire, remplie (`fill`)
    /// et/ou tracée (`stroke`). La brique des icônes et du dessin personnalisé.
    Path {
        path: Path,
        /// Couleur de remplissage intérieur (`None` = pas de remplissage).
        fill: Option<Color>,
        /// Contour (couleur + épaisseur), `None` = pas de contour.
        stroke: Option<Stroke>,
        /// Rectangle de découpe.
        clip: Rect,
        /// Identité du widget émetteur.
        owner: u64,
    },
    /// Une **image** bitmap échantillonnée dans un rectangle de destination.
    Image {
        /// Handle partagé vers les pixels (cache GPU par [`crate::ImageData::id`]).
        image: ImageHandle,
        /// Rectangle de destination (déjà ajusté selon le [`crate::BoxFit`]).
        rect: Rect,
        /// Sous-région de la texture échantillonnée, en `0..1` (rognage `Cover`).
        uv: Rect,
        /// Teinte multiplicative (blanc = inchangé ; alpha pour le fondu).
        tint: Color,
        /// Rectangle de découpe.
        clip: Rect,
        /// Identité du widget émetteur.
        owner: u64,
    },
    /// Un **calque** : un sous-groupe de primitives composité **d'un bloc** à
    /// `opacity`. Rendu à part sur une texture (pleine opacité) puis composité —
    /// l'alpha de groupe est ainsi correct (pas de double-superposition là où les
    /// primitives internes se chevauchent), comme le `saveLayer`/`Opacity` de
    /// Flutter.
    Layer {
        /// Les primitives du groupe (coordonnées absolues, comme la scène mère).
        primitives: Vec<Primitive>,
        /// Opacité de groupe appliquée au calque entier (`0..1`).
        opacity: f32,
        /// Rectangle de découpe du calque.
        clip: Rect,
        /// Transformation affine (rotation) appliquée au compositing. `None` =
        /// calque simplement composité à sa position (opacité de groupe).
        transform: Option<LayerTransform>,
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
            Primitive::RichText { owner, .. } => *owner,
            Primitive::Path { owner, .. } => *owner,
            Primitive::Image { owner, .. } => *owner,
            Primitive::Layer { owner, .. } => *owner,
        }
    }

    /// Met la **géométrie** à l'échelle par `factor` (position, taille, rayon,
    /// bordure, flou, découpe, taille de police). Couleurs et texte inchangés.
    /// Sert à convertir une scène logique en scène physique (DPI).
    pub fn scaled(&self, factor: f32) -> Primitive {
        match self.clone() {
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
                rect: rect.scale(factor),
                color,
                color2,
                gradient_dir,
                radius: radius.scale(factor),
                border_width: border_width * factor,
                border_color,
                blur: blur * factor,
                clip: clip.scale(factor),
                owner,
            },
            Primitive::Text {
                position,
                text,
                size,
                color,
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                clip,
                owner,
            } => Primitive::Text {
                position: position.scale(factor),
                text,
                size: size * factor,
                color,
                weight,
                italic,
                max_width: max_width.map(|w| w * factor),
                decoration,
                decoration_color,
                clip: clip.scale(factor),
                owner,
            },
            Primitive::RichText {
                position,
                mut runs,
                max_width,
                clip,
                owner,
            } => {
                for run in &mut runs {
                    run.size *= factor;
                }
                Primitive::RichText {
                    position: position.scale(factor),
                    runs,
                    max_width: max_width.map(|w| w * factor),
                    clip: clip.scale(factor),
                    owner,
                }
            }
            Primitive::Path {
                path,
                fill,
                stroke,
                clip,
                owner,
            } => Primitive::Path {
                path: path.scaled(factor),
                fill,
                stroke: stroke.map(|s| Stroke::new(s.color, s.width * factor)),
                clip: clip.scale(factor),
                owner,
            },
            Primitive::Image {
                image,
                rect,
                uv,
                tint,
                clip,
                owner,
            } => Primitive::Image {
                image,
                rect: rect.scale(factor),
                // L'UV est en 0..1 : indépendant de l'échelle.
                uv,
                tint,
                clip: clip.scale(factor),
                owner,
            },
            Primitive::Layer {
                primitives,
                opacity,
                clip,
                transform,
                owner,
            } => Primitive::Layer {
                primitives: primitives.iter().map(|p| p.scaled(factor)).collect(),
                opacity,
                clip: clip.scale(factor),
                transform: transform.map(|t| t.scaled(factor)),
                owner,
            },
        }
    }

    /// Décale la **géométrie** de `(dx, dy)` (position, découpe) — couleurs,
    /// tailles et texte inchangés. Combiné à [`Primitive::scaled`], sert à mettre
    /// un sous-arbre à l'échelle **autour d'un pivot** :
    /// `p.scaled(f).translated(pivot.x * (1 - f), pivot.y * (1 - f))`.
    pub fn translated(&self, dx: f32, dy: f32) -> Primitive {
        match self.clone() {
            Primitive::Rect { rect, color, color2, gradient_dir, radius, border_width, border_color, blur, clip, owner } => {
                Primitive::Rect {
                    rect: rect.translate(dx, dy),
                    color,
                    color2,
                    gradient_dir,
                    radius,
                    border_width,
                    border_color,
                    blur,
                    clip: clip.translate(dx, dy),
                    owner,
                }
            }
            Primitive::Text { position, text, size, color, weight, italic, max_width, decoration, decoration_color, clip, owner } => {
                Primitive::Text {
                    position: Point::new(position.x + dx, position.y + dy),
                    text,
                    size,
                    color,
                    weight,
                    italic,
                    max_width,
                    decoration,
                    decoration_color,
                    clip: clip.translate(dx, dy),
                    owner,
                }
            }
            Primitive::RichText { position, runs, max_width, clip, owner } => {
                Primitive::RichText {
                    position: Point::new(position.x + dx, position.y + dy),
                    runs,
                    max_width,
                    clip: clip.translate(dx, dy),
                    owner,
                }
            }
            Primitive::Path { path, fill, stroke, clip, owner } => {
                Primitive::Path {
                    path: path.translated(dx, dy),
                    fill,
                    stroke,
                    clip: clip.translate(dx, dy),
                    owner,
                }
            }
            Primitive::Image { image, rect, uv, tint, clip, owner } => {
                Primitive::Image {
                    image,
                    rect: rect.translate(dx, dy),
                    uv,
                    tint,
                    clip: clip.translate(dx, dy),
                    owner,
                }
            }
            Primitive::Layer { primitives, opacity, clip, transform, owner } => {
                Primitive::Layer {
                    primitives: primitives.iter().map(|p| p.translated(dx, dy)).collect(),
                    opacity,
                    clip: clip.translate(dx, dy),
                    transform: transform.map(|t| t.translated(dx, dy)),
                    owner,
                }
            }
        }
    }

    /// Met la géométrie à l'échelle par `factor` **autour de `pivot`** (le pivot
    /// reste fixe) : `pos' = pivot + (pos - pivot) * factor`. Tailles, police,
    /// rayons et traits suivent l'échelle.
    pub fn scaled_about(&self, pivot: Point, factor: f32) -> Primitive {
        self.scaled(factor)
            .translated(pivot.x * (1.0 - factor), pivot.y * (1.0 - factor))
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

    /// Rectangle de découpe courant (pour l'intersecter avec des bornes locales).
    pub fn current_clip(&self) -> Rect {
        self.current_clip
    }

    /// Fixe l'identité du widget émetteur des primitives suivantes.
    pub fn set_owner(&mut self, owner: u64) {
        self.current_owner = owner;
    }

    /// Rajoute une primitive **déjà formée** (découpe et propriétaire déjà
    /// baked dans la primitive). Sert à rejouer un sous-arbre mis en cache
    /// (frontière de repaint) tel quel, sans le repeindre.
    pub fn push_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// Retire et renvoie les primitives à partir de l'index `start` (ordre
    /// conservé). Sert à **envelopper** un sous-arbre déjà peint dans un calque
    /// ([`Primitive::Layer`]) : on peint le sous-arbre, puis on déplace sa plage
    /// de primitives dans un calque (opacité de groupe).
    pub fn split_off(&mut self, start: usize) -> Vec<Primitive> {
        self.primitives.split_off(start)
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
                weight,
                italic,
                max_width,
                decoration,
                decoration_color,
                clip,
                owner,
            } => Primitive::Text {
                position,
                text,
                size,
                color: color.fade(opacity),
                weight,
                italic,
                max_width,
                decoration,
                decoration_color: decoration_color.map(|c| c.fade(opacity)),
                clip,
                owner,
            },
            Primitive::RichText {
                position,
                mut runs,
                max_width,
                clip,
                owner,
            } => {
                for run in &mut runs {
                    run.color = run.color.fade(opacity);
                    run.decoration_color = run.decoration_color.map(|c| c.fade(opacity));
                }
                Primitive::RichText {
                    position,
                    runs,
                    max_width,
                    clip,
                    owner,
                }
            }
            Primitive::Path {
                path,
                fill,
                stroke,
                clip,
                owner,
            } => Primitive::Path {
                path,
                fill: fill.map(|c| c.fade(opacity)),
                stroke: stroke.map(|s| Stroke::new(s.color.fade(opacity), s.width)),
                clip,
                owner,
            },
            Primitive::Image {
                image,
                rect,
                uv,
                tint,
                clip,
                owner,
            } => Primitive::Image {
                image,
                rect,
                uv,
                tint: tint.fade(opacity),
                clip,
                owner,
            },
            Primitive::Layer {
                primitives,
                opacity: group,
                clip,
                transform,
                owner,
            } => Primitive::Layer {
                primitives,
                // Fondre un calque = atténuer son opacité de groupe.
                opacity: group * opacity,
                clip,
                transform,
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
            radius: BorderRadius::ZERO,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute un rectangle avec coins arrondis (uniformes via `f32`, ou par coin
    /// via [`BorderRadius`]) et/ou bordure.
    pub fn draw_rect(
        &mut self,
        rect: Rect,
        color: Color,
        radius: impl Into<BorderRadius>,
        border_width: f32,
        border_color: Color,
    ) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius: radius.into(),
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
        radius: impl Into<BorderRadius>,
        border_width: f32,
        border_color: Color,
    ) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2,
            gradient_dir: dir,
            radius: radius.into(),
            border_width,
            border_color,
            blur: 0.0,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute une ombre douce (rectangle arrondi au bord flou), sans bordure.
    pub fn shadow(
        &mut self,
        rect: Rect,
        color: Color,
        radius: impl Into<BorderRadius>,
        blur: f32,
    ) {
        self.primitives.push(Primitive::Rect {
            rect,
            color,
            color2: color,
            gradient_dir: [0.0, 0.0],
            radius: radius.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            blur,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Remplit un chemin vectoriel d'une couleur unie (règle *non-zero*).
    pub fn fill_path(&mut self, path: &Path, color: Color) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill: Some(color),
            stroke: None,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Trace le contour d'un chemin (couleur + épaisseur), sans remplissage.
    pub fn stroke_path(&mut self, path: &Path, color: Color, width: f32) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill: None,
            stroke: Some(Stroke::new(color, width)),
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute un chemin rempli **et/ou** tracé (les deux passes en une primitive).
    pub fn paint_path(&mut self, path: &Path, fill: Option<Color>, stroke: Option<Stroke>) {
        self.primitives.push(Primitive::Path {
            path: path.clone(),
            fill,
            stroke,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Dessine une image dans `rect`, en échantillonnant la sous-région `uv`
    /// (en `0..1`) et en la teintant par `tint` (blanc = inchangé). Bas niveau :
    /// voir [`Scene::image`] pour l'ajustement automatique par [`crate::BoxFit`].
    pub fn draw_image(&mut self, image: &ImageHandle, rect: Rect, uv: Rect, tint: Color) {
        self.primitives.push(Primitive::Image {
            image: image.clone(),
            rect,
            uv,
            tint,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Dessine une image ajustée dans `rect` selon `fit` (aspect conservé,
    /// letterbox ou rognage), sans teinte.
    pub fn image(&mut self, image: &ImageHandle, rect: Rect, fit: crate::BoxFit) {
        let (dst, uv) = fit.apply(image.size(), rect);
        self.draw_image(image, dst, uv, Color::WHITE);
    }

    /// Compose un **calque** : `build` remplit un sous-groupe de primitives, qui
    /// est ensuite composité **d'un bloc** à `opacity` (`0..1`). Contrairement à
    /// une opacité appliquée primitive par primitive, l'alpha de groupe reste
    /// correct là où les primitives internes se chevauchent (façon `Opacity` de
    /// Flutter). Le calque hérite de la découpe et du propriétaire courants.
    pub fn layer(&mut self, opacity: f32, build: impl FnOnce(&mut Scene)) {
        let mut inner = Scene::new();
        inner.current_clip = self.current_clip;
        inner.current_owner = self.current_owner;
        build(&mut inner);
        self.primitives.push(Primitive::Layer {
            primitives: inner.primitives,
            opacity,
            clip: self.current_clip,
            transform: None,
            owner: self.current_owner,
        });
    }

    /// Ajoute une ligne de texte, ancrée par son coin haut-gauche (graisse
    /// normale, droit). Voir [`Scene::text_styled`] pour la graisse/l'italique.
    pub fn text(&mut self, position: Point, text: impl Into<String>, size: f32, color: Color) {
        self.primitives.push(Primitive::Text {
            position,
            text: text.into(),
            size,
            color,
            weight: FontWeight::Regular,
            italic: false,
            max_width: None,
            decoration: TextDecoration::NONE,
            decoration_color: None,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute du texte **riche** : des runs résolus (styles/couleurs mêlés) mis en
    /// forme d'un seul tenant, ancré par son coin haut-gauche.
    pub fn rich_text(&mut self, position: Point, runs: Vec<TextRun>) {
        self.primitives.push(Primitive::RichText {
            position,
            runs,
            max_width: None,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute un **paragraphe riche** : les runs reviennent à la ligne au-delà de
    /// `max_width` (le repli du rendu suit celui de la mise en page).
    pub fn rich_text_wrapped(&mut self, position: Point, runs: Vec<TextRun>, max_width: f32) {
        self.primitives.push(Primitive::RichText {
            position,
            runs,
            max_width: Some(max_width),
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute une ligne de texte stylée (taille/graisse/italique du [`TextStyle`]).
    /// `color` est la couleur **résolue** (la `color` optionnelle du style ayant
    /// été tranchée par l'appelant, généralement contre le thème).
    pub fn text_styled(
        &mut self,
        position: Point,
        text: impl Into<String>,
        style: &TextStyle,
        color: Color,
    ) {
        self.primitives.push(Primitive::Text {
            position,
            text: text.into(),
            size: style.size,
            color,
            weight: style.weight,
            italic: style.italic,
            max_width: None,
            decoration: style.decoration,
            decoration_color: style.decoration_color,
            clip: self.current_clip,
            owner: self.current_owner,
        });
    }

    /// Ajoute un **paragraphe** : texte stylé qui revient à la ligne au-delà de
    /// `max_width` (le repli du rendu suit celui de la mise en page).
    pub fn text_wrapped(
        &mut self,
        position: Point,
        text: impl Into<String>,
        style: &TextStyle,
        color: Color,
        max_width: f32,
    ) {
        self.primitives.push(Primitive::Text {
            position,
            text: text.into(),
            size: style.size,
            color,
            weight: style.weight,
            italic: style.italic,
            max_width: Some(max_width),
            decoration: style.decoration,
            decoration_color: style.decoration_color,
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

    /// Une copie de la scène avec toute la géométrie mise à l'échelle par
    /// `factor` (conversion logique → physique pour le rendu HiDPI).
    pub fn scaled(&self, factor: f32) -> Scene {
        Scene {
            primitives: self.primitives.iter().map(|p| p.scaled(factor)).collect(),
            current_clip: self.current_clip.scale(factor),
            current_owner: self.current_owner,
        }
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
                radius: BorderRadius::ZERO,
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
    fn scaled_multiplies_geometry_not_colors() {
        let mut scene = Scene::new();
        scene.draw_rect(Rect::new(2.0, 4.0, 10.0, 20.0), Color::rgb(1.0, 0.0, 0.0), 3.0, 1.0, Color::WHITE);
        scene.text(Point::new(5.0, 6.0), "hi", 18.0, Color::BLACK);

        let big = scene.scaled(2.0);
        match &big.primitives()[0] {
            Primitive::Rect { rect, radius, border_width, color, .. } => {
                assert_eq!(*rect, Rect::new(4.0, 8.0, 20.0, 40.0));
                assert_eq!(*radius, BorderRadius::uniform(6.0));
                assert_eq!(*border_width, 2.0);
                assert_eq!(*color, Color::rgb(1.0, 0.0, 0.0)); // couleur inchangée
            }
            _ => panic!("attendu un rectangle"),
        }
        match &big.primitives()[1] {
            Primitive::Text { position, size, text, .. } => {
                assert_eq!(*position, Point::new(10.0, 12.0));
                assert_eq!(*size, 36.0);
                assert_eq!(text, "hi"); // texte inchangé
            }
            _ => panic!("attendu du texte"),
        }
    }

    #[test]
    fn layer_captures_subprimitives_and_opacity() {
        let mut scene = Scene::new();
        scene.set_owner(7);
        scene.set_clip(Rect::new(0.0, 0.0, 50.0, 50.0));
        scene.layer(0.5, |inner| {
            inner.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::WHITE);
            inner.fill_rect(Rect::new(5.0, 5.0, 10.0, 10.0), Color::BLACK);
        });
        assert_eq!(scene.len(), 1);
        match &scene.primitives()[0] {
            Primitive::Layer { primitives, opacity, clip, owner, .. } => {
                assert_eq!(primitives.len(), 2);
                assert_eq!(*opacity, 0.5);
                assert_eq!(*clip, Rect::new(0.0, 0.0, 50.0, 50.0));
                assert_eq!(*owner, 7);
            }
            _ => panic!("attendu un calque"),
        }
    }

    #[test]
    fn fading_a_layer_scales_its_group_opacity() {
        let mut scene = Scene::new();
        scene.layer(0.8, |inner| inner.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::WHITE));
        let layer = scene.primitives()[0].clone();
        let mut target = Scene::new();
        target.push_faded(&layer, 0.5);
        match &target.primitives()[0] {
            Primitive::Layer { opacity, .. } => assert!((*opacity - 0.4).abs() < 1e-6),
            _ => panic!("attendu un calque"),
        }
    }

    #[test]
    fn scaling_a_layer_scales_its_children() {
        let mut scene = Scene::new();
        scene.layer(1.0, |inner| inner.fill_rect(Rect::new(2.0, 3.0, 4.0, 5.0), Color::WHITE));
        let big = scene.scaled(2.0);
        match &big.primitives()[0] {
            Primitive::Layer { primitives, .. } => match &primitives[0] {
                Primitive::Rect { rect, .. } => assert_eq!(*rect, Rect::new(4.0, 6.0, 8.0, 10.0)),
                _ => panic!("attendu un rectangle"),
            },
            _ => panic!("attendu un calque"),
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
