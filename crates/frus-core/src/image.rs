//! Bitmap images: decoded, shared pixels, and how they fit inside a box.
//!
//! `frus-core` neither decodes (PNG/JPEG and friends) nor uploads to the GPU: it
//! holds only the **raw pixels** ([`ImageData`], RGBA sRGB) behind a shared handle
//! ([`ImageHandle`]), plus the **fitting** logic ([`BoxFit`]). Decoding lives in a
//! dedicated layer; uploading lives in `frus-gpu`, which caches the texture by
//! [`ImageData::id`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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

/// The process-wide store of images already turned into pixels, keyed by where their
/// bytes live.
///
/// Decoding a PNG is not free and a view is rebuilt every frame, so an image resolved
/// in `view` must be resolved **once** rather than sixty times a second. That is what
/// this is for, and it is a process-wide store for the same reason the font registry is
/// one: the same asset shown on three screens is one image, not three.
///
/// A failure is cached too. A file that is not a PNG will not become one on the next
/// frame, and retrying the decode every frame would turn one broken asset into a
/// permanent cost.
static CACHE: Mutex<Option<HashMap<usize, Result<ImageHandle, String>>>> = Mutex::new(None);

/// Resolves `bytes` to pixels **once**, returning the same handle on every later call.
///
/// The key is the **address** of the slice rather than its contents. That is exact for
/// what this is for — `include_bytes!` gives a `&'static [u8]` whose address is fixed
/// for the life of the process, and two distinct assets are two distinct statics — and
/// it costs a pointer comparison, where hashing the bytes would cost re-reading the
/// whole file on every frame that shows it.
///
/// The `'static` bound is what makes that sound: bytes that can be freed could have
/// their address reused by something else, and the cache would hand back the wrong
/// picture. An application holding runtime bytes decodes them itself and keeps the
/// [`ImageHandle`].
///
/// `load` runs only on the first call for a given slice, and its failure is remembered.
pub fn cached(
    bytes: &'static [u8],
    load: impl FnOnce(&[u8]) -> Result<ImageData, String>,
) -> Result<ImageHandle, String> {
    let key = bytes.as_ptr() as usize;
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some(found) = cache.get(&key) {
        return found.clone();
    }
    let resolved = load(bytes).map(ImageData::into_handle);
    cache.insert(key, resolved.clone());
    resolved
}

/// Empties the store. For tests that want a decode to happen again; an application has
/// no reason to call it, since the entries are one per embedded asset and bounded by
/// how many the binary carries.
pub fn forget_cached_images() {
    if let Ok(mut guard) = CACHE.lock() {
        *guard = None;
    }
}

/// What is known about an image that has to be **fetched** before it can be shown.
///
/// Three states rather than two, because "not here yet" and "will never be here" are
/// different answers and an interface shows them differently: one is a placeholder, the
/// other is a message.
#[derive(Clone, Debug)]
pub enum Fetched {
    /// In flight. Ask again on a later frame.
    Loading,
    /// Here.
    Ready(ImageHandle),
    /// It will not arrive, and this is why.
    Failed(String),
}

/// Hands the bytes at `url` to the callback, once, from wherever suits the platform.
///
/// A **function pointer** and not a closure: this is registered once for the process, and
/// a plain `fn` needs no allocation, no lifetime and no `Sync` wrapper to store.
pub type ImageFetcher = fn(&str, Box<dyn FnOnce(Result<Vec<u8>, String>) + Send + 'static>);

static FETCHER: Mutex<Option<ImageFetcher>> = Mutex::new(None);
static FETCHED: Mutex<Option<HashMap<String, Fetched>>> = Mutex::new(None);

/// Names the thing that gets bytes over the network.
///
/// The widget layer cannot do this itself and should not: it has no runtime, no socket
/// and no dependency on the shell — the dependency runs the other way. So the shell,
/// which has all three, says how, and the widget layer only asks. It is the shape the
/// image **decoder** took a step earlier, for the same reason.
///
/// An application using the framework's own shell never calls this: the shell registers
/// its `fetch_bytes` on the way up. One embedding frus in a host that already has an HTTP
/// client can point this at that client instead.
pub fn set_image_fetcher(fetcher: ImageFetcher) {
    if let Ok(mut guard) = FETCHER.lock() {
        *guard = Some(fetcher);
    }
}

/// What is known about the image at `url`, starting the fetch if this is the first ask.
///
/// Called from a view, which runs every frame: the **first** call starts the work and
/// says `Loading`, and every later one is a lookup. That is what makes it safe to write
/// in a `view` at all — a fetch per frame would be sixty requests a second for one
/// picture.
///
/// A failure is remembered like any other, and for the same reason as milestone 372's:
/// a URL that 404s will 404 again, and retrying every frame turns one bad link into a
/// permanent load on somebody's server.
pub fn fetched(url: &str, decode: fn(&[u8]) -> Result<ImageData, String>) -> Fetched {
    let mut guard = FETCHED.lock().unwrap_or_else(|e| e.into_inner());
    let store = guard.get_or_insert_with(HashMap::new);
    if let Some(known) = store.get(url) {
        return known.clone();
    }
    let Some(fetcher) = *FETCHER.lock().unwrap_or_else(|e| e.into_inner()) else {
        // No fetcher: a build without the network, or a host that never named one.
        // Saying so is an answer; hanging on `Loading` for ever is not.
        let failed = Fetched::Failed("no image fetcher registered".to_string());
        store.insert(url.to_string(), failed.clone());
        return failed;
    };
    store.insert(url.to_string(), Fetched::Loading);
    // The lock goes **before** the call. A fetcher is free to answer on this thread —
    // a cache hit in a host client, a test double — and its callback locks the same
    // store, which would be a deadlock against a guard still held here.
    drop(guard);

    let key = url.to_string();
    fetcher(
        url,
        Box::new(move |result| {
            let outcome = match result {
                Ok(bytes) => match decode(&bytes) {
                    Ok(data) => Fetched::Ready(data.into_handle()),
                    Err(why) => Fetched::Failed(why),
                },
                Err(why) => Fetched::Failed(why),
            };
            if let Ok(mut guard) = FETCHED.lock() {
                guard.get_or_insert_with(HashMap::new).insert(key, outcome);
            }
        }),
    );
    Fetched::Loading
}

/// How many images are in flight right now.
///
/// The interface has to keep drawing while any of them are, or the frame that would show
/// the picture never happens. A count rather than a flag on the widget, because the
/// application may well have taken the image **out** of the tree while it loads — that
/// is what showing a placeholder means — and the work is still going on.
///
/// Counted off the store rather than kept in a separate tally beside it. A tally is a
/// second copy of the same fact and the two can disagree: incremented here and
/// decremented in a callback, it survives a [`forget_fetched_images`] that empties the
/// store, and it can be decremented below zero by a callback that arrives after one.
/// Either way the interface is left redrawing for ever over work that finished. The
/// store already knows — the entries that say `Loading` **are** the ones in flight —
/// and there are only ever as many entries as the application has network images.
pub fn images_in_flight() -> usize {
    FETCHED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map_or(0, |store| {
            store
                .values()
                .filter(|state| matches!(state, Fetched::Loading))
                .count()
        })
}

/// Forgets every fetched image and every remembered failure. For tests; see
/// [`forget_cached_images`].
pub fn forget_fetched_images() {
    if let Ok(mut guard) = FETCHED.lock() {
        *guard = None;
    }
}

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
