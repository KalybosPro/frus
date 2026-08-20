//! The images the demo embeds.

use crate::prelude::*;
use frus_widgets::asset;

/// The demo logo, embedded from a PNG and decoded once for the whole process.
///
/// This used to be a `OnceLock`, an explicit `frus_image::decode`, a `map` and an
/// `unwrap_or_else` — five lines of caching the demo had to write because the framework
/// would not. Milestone 372 moved that behind [`asset!`], and the fallback below is now
/// what it says it is: the picture shown when the file will not decode, chosen by this
/// application rather than by the widget.
pub(crate) fn demo_logo() -> Image {
    let logo = asset!("../assets/logo.png");
    match logo.error() {
        None => logo,
        Some(_) => Image::new(fallback_gradient()),
    }
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
