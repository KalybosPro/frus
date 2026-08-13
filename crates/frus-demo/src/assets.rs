//! The images the demo embeds, decoded once and shared for the whole process.

use crate::prelude::*;

/// The demo logo, **decoded** from an embedded PNG (milestone 91) and shared across the whole
/// process through a `OnceLock` — decoded once, then cached by identity on the renderer's
/// side. Falls back to a generated gradient if the decoding fails (robustness).
pub(crate) fn demo_image() -> ImageHandle {
    use std::sync::OnceLock;
    static IMG: OnceLock<ImageHandle> = OnceLock::new();
    IMG.get_or_init(|| {
        frus_image::decode(include_bytes!("../assets/logo.png"))
            .map(ImageData::into_handle)
            .unwrap_or_else(|_| fallback_gradient())
    })
    .clone()
}

/// A generated 64×64 gradient — the fallback when decoding the PNG fails.
pub(crate) fn fallback_gradient() -> ImageHandle {
    const W: u32 = 64;
    const H: u32 = 64;
    let mut rgba = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H {
        for x in 0..W {
            rgba.push((x * 255 / (W - 1)) as u8);
            rgba.push((y * 255 / (H - 1)) as u8);
            rgba.push(160u8);
            rgba.push(255u8);
        }
    }
    ImageData::from_rgba(W, H, rgba).into_handle()
}
