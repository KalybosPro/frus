//! Generates the demo's PNG logo on **standard output** — a reproducible provenance
//! for the binary asset, rather than an opaque file committed by hand.
//!
//! ```text
//! cargo run -p frus-image --example gen_logo > crates/frus-demo/assets/logo.png
//! ```

use std::io::{Cursor, Write};

fn main() {
    const W: u32 = 96;
    const H: u32 = 96;
    let (cx, cy) = (W as f32 / 2.0, H as f32 / 2.0);

    let mut img = image::RgbaImage::new(W, H);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        // A diagonal gradient (R horizontal, G vertical) plus a lighter central disc.
        let r = (x * 255 / (W - 1)) as u8;
        let g = (y * 255 / (H - 1)) as u8;
        let b = if dist < 34.0 { 235 } else { 80 };
        *px = image::Rgba([r, g, b, 255]);
    }

    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encodage PNG");
    std::io::stdout()
        .write_all(&bytes)
        .expect("writing to stdout");
}
