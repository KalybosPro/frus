//! Images bitmap : pixels décodés, partagés, et leur ajustement dans une boîte.
//!
//! `frus-core` ne sait ni décoder (PNG/JPEG…) ni téléverser sur le GPU : il ne
//! détient que les **pixels bruts** ([`ImageData`], RGBA sRGB) derrière un
//! handle partagé ([`ImageHandle`]) et la logique d'**ajustement** ([`BoxFit`]).
//! Le décodage vit dans une couche dédiée ; le téléversement dans `frus-gpu`,
//! qui met en cache la texture par [`ImageData::id`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{Rect, Size};

/// Compteur d'identités : chaque [`ImageData`] reçoit un id unique et stable,
/// clé de cache côté GPU (évite de re-téléverser les mêmes pixels à chaque frame).
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Les **pixels** d'une image : RGBA 8 bits, sRGB, rangée par rangée depuis le
/// coin haut-gauche (`width * height * 4` octets). Immuable une fois construit.
#[derive(Debug)]
pub struct ImageData {
    id: u64,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl ImageData {
    /// Construit une image à partir de pixels RGBA bruts. Panique si la longueur
    /// ne vaut pas `width * height * 4`.
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert_eq!(
            rgba.len(),
            (width as usize) * (height as usize) * 4,
            "les pixels RGBA doivent faire width*height*4 octets"
        );
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            rgba,
        }
    }

    /// Enveloppe l'image dans un handle partagé (clone bon marché, cache stable).
    pub fn into_handle(self) -> ImageHandle {
        Arc::new(self)
    }

    /// Identité unique et stable (clé de cache GPU).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Largeur en pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Hauteur en pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Taille (en pixels) sous forme de [`Size`].
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    /// Les octets RGBA (sRGB), rangée par rangée.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// Deux images sont « égales » si elles partagent la même identité (même
/// ressource), sans comparer les pixels — clé de cache et égalité de scène bon
/// marché.
impl PartialEq for ImageData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ImageData {}

/// Un handle d'image **partagé** : clone bon marché (compteur de références),
/// stocké tel quel dans une [`crate::Primitive::Image`].
pub type ImageHandle = Arc<ImageData>;

/// Comment ajuster une image dans sa boîte de destination (façon `BoxFit` de
/// Flutter / `object-fit` CSS).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoxFit {
    /// Étire pour remplir la boîte (l'aspect n'est **pas** conservé).
    Fill,
    /// Le plus grand agrandissement qui **tient** dans la boîte (letterbox).
    #[default]
    Contain,
    /// Le plus petit agrandissement qui **couvre** la boîte (rognage).
    Cover,
    /// Ajuste sur la **largeur** (peut déborder en hauteur).
    FitWidth,
    /// Ajuste sur la **hauteur** (peut déborder en largeur).
    FitHeight,
    /// Taille naturelle, centrée (ni agrandi ni réduit).
    None,
    /// Comme [`BoxFit::Contain`] mais **jamais agrandie** (réduit seulement).
    ScaleDown,
}

impl BoxFit {
    /// Calcule le rectangle de **destination** (dans/autour de `dst`) et le
    /// rectangle **UV** (sous-région de la texture, en `0..1`) pour dessiner une
    /// image de taille `src` selon ce mode. `dst` non rogné : letterbox → dst
    /// rétréci + UV plein ; rognage → dst plein + UV réduit.
    pub fn apply(self, src: Size, dst: Rect) -> (Rect, Rect) {
        let full_uv = Rect::new(0.0, 0.0, 1.0, 1.0);
        if src.width <= 0.0 || src.height <= 0.0 || dst.width <= 0.0 || dst.height <= 0.0 {
            return (dst, full_uv);
        }
        let sx = dst.width / src.width;
        let sy = dst.height / src.height;

        // Letterbox : image mise à `scale`, centrée dans `dst`, UV plein.
        let letterbox = |scale: f32| {
            let w = src.width * scale;
            let h = src.height * scale;
            let x = dst.x + (dst.width - w) * 0.5;
            let y = dst.y + (dst.height - h) * 0.5;
            (Rect::new(x, y, w, h), full_uv)
        };

        match self {
            BoxFit::Fill => (dst, full_uv),
            BoxFit::Contain => letterbox(sx.min(sy)),
            BoxFit::FitWidth => letterbox(sx),
            BoxFit::FitHeight => letterbox(sy),
            BoxFit::None => letterbox(1.0),
            BoxFit::ScaleDown => letterbox(sx.min(sy).min(1.0)),
            BoxFit::Cover => {
                // L'image couvre `dst` ; on rogne via l'UV (centré).
                let scale = sx.max(sy);
                let scaled_w = src.width * scale;
                let scaled_h = src.height * scale;
                let uv_w = (dst.width / scaled_w).min(1.0);
                let uv_h = (dst.height / scaled_h).min(1.0);
                let uv = Rect::new((1.0 - uv_w) * 0.5, (1.0 - uv_h) * 0.5, uv_w, uv_h);
                (dst, uv)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32) -> ImageData {
        ImageData::from_rgba(w, h, vec![0u8; (w * h * 4) as usize])
    }

    #[test]
    fn ids_are_unique_and_stable() {
        let a = img(1, 1);
        let b = img(1, 1);
        assert_ne!(a.id(), b.id());
        assert_eq!(a.id(), a.id());
        assert_ne!(a, b); // égalité par identité, pas par pixels
    }

    #[test]
    fn fill_uses_the_whole_box_and_full_uv() {
        let (dst, uv) = BoxFit::Fill.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(dst, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn contain_letterboxes_preserving_aspect() {
        // Source carrée dans une boîte large → hauteur pleine, centrée en largeur.
        let (dst, uv) = BoxFit::Contain.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(dst, Rect::new(30.0, 0.0, 40.0, 40.0));
        assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn cover_fills_the_box_and_crops_uv() {
        // Source carrée dans une boîte large → couvre tout, rogne en hauteur (UV).
        let (dst, uv) = BoxFit::Cover.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(dst, Rect::new(0.0, 0.0, 100.0, 40.0));
        // scale = max(10, 4) = 10 → image 100×100 ; visible = 40/100 = 0.4 de hauteur.
        assert_eq!(uv.width, 1.0);
        assert!((uv.height - 0.4).abs() < 1e-6);
        assert!((uv.y - 0.3).abs() < 1e-6);
    }
}
