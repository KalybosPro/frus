//! Bitmap images: decoded, shared pixels, and how they fit inside a box.
//!
//! `frus-core` neither decodes (PNG/JPEG and friends) nor uploads to the GPU: it
//! holds only the **raw pixels** ([`ImageData`], RGBA sRGB) behind a shared handle
//! ([`ImageHandle`]), plus the **fitting** logic ([`BoxFit`]). Decoding lives in a
//! dedicated layer; uploading lives in `frus-gpu`, which caches the texture by
//! [`ImageData::id`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{Alignment, Rect, Size};

/// Identity counter: every [`ImageData`] gets a unique, stable id, which is the
/// GPU-side cache key — it keeps the same pixels from being re-uploaded each frame.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// An image's **pixels**: 8-bit RGBA, sRGB, row by row from the top-left corner
/// (`width * height * 4` bytes). Immutable once built.
#[derive(Debug)]
pub struct ImageData {
    id: u64,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl ImageData {
    /// Builds an image from raw RGBA pixels. Panics if the length is not
    /// `width * height * 4`.
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert_eq!(
            rgba.len(),
            (width as usize) * (height as usize) * 4,
            "RGBA pixels must be width*height*4 bytes"
        );
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            rgba,
        }
    }

    /// Wraps the image in a shared handle (cheap to clone, stable for caching).
    pub fn into_handle(self) -> ImageHandle {
        Arc::new(self)
    }

    /// Unique, stable identity (the GPU cache key).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Size in pixels, as a [`Size`].
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    /// The RGBA bytes (sRGB), row by row.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// Two images are "equal" when they share the same identity — the same resource —
/// without comparing pixels. That keeps cache lookups and scene equality cheap.
impl PartialEq for ImageData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ImageData {}

/// A **shared** image handle: cheap to clone (reference counted), and stored as-is
/// inside a [`crate::Primitive::Image`].
pub type ImageHandle = Arc<ImageData>;

/// How an image is fitted into its destination box — the same set of modes as the
/// CSS `object-fit` property.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoxFit {
    /// Stretches to fill the box; the aspect ratio is **not** preserved.
    Fill,
    /// The largest scale that still **fits** inside the box (letterbox).
    #[default]
    Contain,
    /// The smallest scale that **covers** the box (cropping).
    Cover,
    /// Fits to the **width**; may overflow vertically.
    FitWidth,
    /// Fits to the **height**; may overflow horizontally.
    FitHeight,
    /// Natural size, centred — neither scaled up nor down.
    None,
    /// Like [`BoxFit::Contain`], but **never scaled up** — only down.
    ScaleDown,
}

impl BoxFit {
    /// Computes the **destination** rectangle (inside or around `dst`) and the **UV**
    /// rectangle (the sub-region of the texture, in `0..1`) for drawing an image of
    /// size `src` in this mode. `dst` is never cropped: letterboxing shrinks `dst`
    /// and keeps full UV; cropping keeps `dst` full and shrinks the UV.
    pub fn apply(self, src: Size, dst: Rect) -> (Rect, Rect) {
        self.apply_aligned(src, dst, Alignment::CENTER)
    }

    /// [`BoxFit::apply`] with a say in **where** the spare room goes: which part of the
    /// box a letterboxed image sits in, and which part of the image a cropping one keeps.
    ///
    /// Centre is the reference's default and what `apply` uses. It matters as soon as an
    /// image is not the shape of its box: a portrait cropped to a banner should usually
    /// keep its top, not its middle.
    pub fn apply_aligned(self, src: Size, dst: Rect, align: Alignment) -> (Rect, Rect) {
        let full_uv = Rect::new(0.0, 0.0, 1.0, 1.0);
        if src.width <= 0.0 || src.height <= 0.0 || dst.width <= 0.0 || dst.height <= 0.0 {
            return (dst, full_uv);
        }
        let sx = dst.width / src.width;
        let sy = dst.height / src.height;
        let (fx, fy) = (align.fraction_x(), align.fraction_y());

        // Letterbox: the image at `scale`, placed inside `dst` by the alignment, with
        // full UV.
        let letterbox = |scale: f32| {
            let w = src.width * scale;
            let h = src.height * scale;
            let x = dst.x + (dst.width - w) * fx;
            let y = dst.y + (dst.height - h) * fy;
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
                // The image covers `dst`; the crop happens in the UV, centred.
                let scale = sx.max(sy);
                let scaled_w = src.width * scale;
                let scaled_h = src.height * scale;
                let uv_w = (dst.width / scaled_w).min(1.0);
                let uv_h = (dst.height / scaled_h).min(1.0);
                // The crop travels the other way: aligning the image to the top means
                // keeping the **top** of it, which is the low end of the UV.
                let uv = Rect::new((1.0 - uv_w) * fx, (1.0 - uv_h) * fy, uv_w, uv_h);
                (dst, uv)
            }
        }
    }

    /// The `(sx, sy)` scale factors that fit content of size `src` into a box of
    /// size `dst` in this mode — the building block of `FittedBox`, where the
    /// content, unlike a sampled image, is **scaled** and then centred. `Fill`
    /// stretches per axis; every other mode is **uniform**, preserving the aspect
    /// ratio. A degenerate `src` yields `(1, 1)`.
    pub fn scale(self, src: Size, dst: Size) -> (f32, f32) {
        if src.width <= 0.0 || src.height <= 0.0 {
            return (1.0, 1.0);
        }
        let sx = dst.width / src.width;
        let sy = dst.height / src.height;
        match self {
            BoxFit::Fill => (sx, sy),
            BoxFit::Contain => {
                let s = sx.min(sy);
                (s, s)
            }
            BoxFit::Cover => {
                let s = sx.max(sy);
                (s, s)
            }
            BoxFit::FitWidth => (sx, sx),
            BoxFit::FitHeight => (sy, sy),
            BoxFit::None => (1.0, 1.0),
            BoxFit::ScaleDown => {
                let s = sx.min(sy).min(1.0);
                (s, s)
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
        assert_ne!(a, b); // equality is by identity, not by pixels
    }

    #[test]
    fn fill_uses_the_whole_box_and_full_uv() {
        let (dst, uv) = BoxFit::Fill.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(dst, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn contain_letterboxes_preserving_aspect() {
        // A square source in a wide box → full height, centred horizontally.
        let (dst, uv) =
            BoxFit::Contain.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(dst, Rect::new(30.0, 0.0, 40.0, 40.0));
        assert_eq!(uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn scale_fits_content_per_mode() {
        let src = Size::new(40.0, 20.0); // wide
        let dst = Size::new(200.0, 200.0);
        // Fill: per axis.
        assert_eq!(BoxFit::Fill.scale(src, dst), (5.0, 10.0));
        // Contain: the smaller factor (it fits) → uniform.
        assert_eq!(BoxFit::Contain.scale(src, dst), (5.0, 5.0));
        // Cover: the larger factor (it covers) → uniform.
        assert_eq!(BoxFit::Cover.scale(src, dst), (10.0, 10.0));
        // FitWidth / FitHeight follow a single axis, uniformly.
        assert_eq!(BoxFit::FitWidth.scale(src, dst), (5.0, 5.0));
        assert_eq!(BoxFit::FitHeight.scale(src, dst), (10.0, 10.0));
        // None does not scale; ScaleDown only ever shrinks.
        assert_eq!(BoxFit::None.scale(src, dst), (1.0, 1.0));
        assert_eq!(BoxFit::ScaleDown.scale(src, dst), (1.0, 1.0));
        // ScaleDown does shrink when the box is smaller.
        assert_eq!(
            BoxFit::ScaleDown.scale(src, Size::new(20.0, 20.0)),
            (0.5, 0.5)
        );
        // Degenerate source → neutral.
        assert_eq!(BoxFit::Cover.scale(Size::new(0.0, 10.0), dst), (1.0, 1.0));
    }

    #[test]
    fn cover_fills_the_box_and_crops_uv() {
        // A square source in a wide box → covers everything, crops vertically (UV).
        let (dst, uv) =
            BoxFit::Cover.apply(Size::new(10.0, 10.0), Rect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(dst, Rect::new(0.0, 0.0, 100.0, 40.0));
        // scale = max(10, 4) = 10 → a 100×100 image; visible = 40/100 = 0.4 of its height.
        assert_eq!(uv.width, 1.0);
        assert!((uv.height - 0.4).abs() < 1e-6);
        assert!((uv.y - 0.3).abs() < 1e-6);
    }
}
