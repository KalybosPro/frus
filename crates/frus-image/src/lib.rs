//! `frus-image` — image **decoding** (PNG/JPEG) into [`frus_core::ImageData`], at the
//! file's own size ([`decode`]) or at the size it is shown at ([`decode_at`]).
//!
//! A thin, **optional** layer: `frus-core` is dependency-free and holds nothing but
//! raw pixels; this crate adds the decoder dependency (`image`) to turn file bytes
//! into an `ImageData` ready to upload. Applications use it to load their assets
//! (`decode(include_bytes!("logo.png"))`) without `frus-widgets` or `frus-core`
//! inheriting the decoder's dependency tree.

#![deny(missing_docs)]

// The README's example is compiled and run by `cargo test --doc`, so it cannot rot.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

use std::error::Error;
use std::fmt;

use frus_core::{DecodeSize, ImageData};

/// A decoding failure: an unrecognised format, or corrupt data.
#[derive(Debug)]
pub struct DecodeError(String);

impl DecodeError {
    /// The underlying error message.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "image decoding failed: {}", self.0)
    }
}

impl Error for DecodeError {}

/// Decodes image bytes (PNG or JPEG, the **format being detected** from the magic
/// bytes) into an sRGB RGBA [`ImageData`]. Every image is converted to RGBA8.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Encode a 2x2 image as PNG, then decode it back.
/// let src = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
/// let mut png = Vec::new();
/// image::DynamicImage::ImageRgba8(src)
///     .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
/// let img = frus_image::decode(&png)?;
/// assert_eq!((img.width(), img.height()), (2, 2));
/// # Ok(()) }
/// ```
pub fn decode(bytes: &[u8]) -> Result<ImageData, DecodeError> {
    let decoded = image::load_from_memory(bytes).map_err(|e| DecodeError(e.to_string()))?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(ImageData::from_rgba(width, height, rgba.into_raw()))
}

/// The width and height of the image in `bytes`, read from its header without decoding
/// it: `None` when the format is not one this crate reads, or the header is broken.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let src = image::RgbaImage::new(40, 30);
/// let mut png = Vec::new();
/// image::DynamicImage::ImageRgba8(src)
///     .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
/// assert_eq!(frus_image::dimensions(&png), Some((40, 30)));
/// # Ok(()) }
/// ```
pub fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// Decodes image bytes **at a size**, smaller than the file's when the place it is shown
/// at needs less: see [`DecodeSize`] for how the size is chosen.
///
/// A JPEG is decoded at 1/2, 1/4 or 1/8 of its size straight from the file, which costs
/// a fraction of a full decode in time and memory, and the rest of the way is a
/// downsampling of that. Any other format is decoded in full, then downsampled **once**,
/// here, rather than on the GPU every frame — the full-size pixels are dropped before
/// this returns.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use frus_core::DecodeSize;
/// let src = image::RgbaImage::from_pixel(400, 300, image::Rgba([0, 128, 255, 255]));
/// let mut png = Vec::new();
/// image::DynamicImage::ImageRgba8(src)
///     .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
/// let thumb = frus_image::decode_at(&png, DecodeSize::new(Some(100), None))?;
/// assert_eq!((thumb.width(), thumb.height()), (100, 75));
/// # Ok(()) }
/// ```
pub fn decode_at(bytes: &[u8], size: DecodeSize) -> Result<ImageData, DecodeError> {
    if size.is_full() {
        return decode(bytes);
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        // A JPEG this decoder cannot scale — a CMYK or a 16-bit one, or one it cannot
        // read at all — takes the general path below.
        if let Some(scaled) = scaled_jpeg(bytes, size) {
            return Ok(scaled);
        }
    }
    let decoded = image::load_from_memory(bytes).map_err(|e| DecodeError(e.to_string()))?;
    let rgba = decoded.to_rgba8();
    drop(decoded);
    let (width, height) = size.target(rgba.width(), rgba.height());
    let rgba = downsample(rgba, width, height);
    Ok(ImageData::from_rgba(width, height, rgba.into_raw()))
}

/// A JPEG decoded at the smallest of its inverse-DCT scales that still covers `size`, and
/// downsampled the rest of the way. `None` when this decoder cannot or need not do it.
fn scaled_jpeg(bytes: &[u8], size: DecodeSize) -> Option<ImageData> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let full = (u32::from(info.width), u32::from(info.height));
    let (width, height) = size.target(full.0, full.1);
    if (width, height) == full {
        return None;
    }
    // The decoder takes a scale once *either* side reaches what is asked; asking for a
    // size at the image's own ratio makes that both.
    decoder.scale(width as u16, height as u16).ok()?;
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (scaled_width, scaled_height) = (u32::from(info.width), u32::from(info.height));
    let rgba: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&l| [l, l, l, 255]).collect(),
        jpeg_decoder::PixelFormat::L16 | jpeg_decoder::PixelFormat::CMYK32 => return None,
    };
    let rgba = image::RgbaImage::from_raw(scaled_width, scaled_height, rgba)?;
    let rgba = downsample(rgba, width, height);
    Some(ImageData::from_rgba(width, height, rgba.into_raw()))
}

/// `rgba` brought down to `width` × `height`, every source pixel counting, so fine detail
/// does not alias.
///
/// A large reduction is mostly **halvings** — each output pixel the mean of four — which
/// are exact for a factor of two and cost one cheap pass each; the filter, as wide as what
/// is left of the reduction, does the last factor of two to four. The filter alone over a
/// twelve-megapixel photograph took 160 ms; the halvings make it a few.
///
/// Translucent pixels are **premultiplied** throughout and divided back at the end: a
/// transparent pixel's colour is not part of the picture, and averaged in as it is it
/// darkens every edge it touches.
fn downsample(rgba: image::RgbaImage, width: u32, height: u32) -> image::RgbaImage {
    if rgba.dimensions() == (width, height) {
        return rgba;
    }
    let translucent = rgba.pixels().any(|p| p[3] < 255);
    let mut img = rgba;
    if translucent {
        for p in img.pixels_mut() {
            let a = u16::from(p[3]);
            for c in 0..3 {
                p[c] = ((u16::from(p[c]) * a + 127) / 255) as u8;
            }
        }
    }
    while img.width() / 2 >= width * 2 && img.height() / 2 >= height * 2 {
        img = halve(&img);
    }
    let mut out =
        image::imageops::resize(&img, width, height, image::imageops::FilterType::Triangle);
    if translucent {
        for p in out.pixels_mut() {
            let a = u16::from(p[3]);
            for c in 0..3 {
                // A fully transparent pixel has no colour to give back, and keeps none.
                if let Some(v) = (u16::from(p[c]) * 255 + a / 2).checked_div(a) {
                    p[c] = v.min(255) as u8;
                }
            }
        }
    }
    out
}

/// Half the size each way, each pixel the mean of the four it covers. An odd last row or
/// column is left out: one pixel in thousands, at the edge.
fn halve(img: &image::RgbaImage) -> image::RgbaImage {
    let (width, height) = (img.width() / 2, img.height() / 2);
    let stride = img.width() as usize * 4;
    let src = img.as_raw();
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height as usize {
        let top = &src[2 * y * stride..][..stride];
        let bottom = &src[(2 * y + 1) * stride..][..stride];
        for x in 0..width as usize {
            let i = x * 8;
            for c in 0..4 {
                let sum = u16::from(top[i + c])
                    + u16::from(top[i + 4 + c])
                    + u16::from(bottom[i + c])
                    + u16::from(bottom[i + 4 + c]);
                out.push(((sum + 2) / 4) as u8);
            }
        }
    }
    image::RgbaImage::from_raw(width, height, out).expect("the size was computed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Encodes a test `RgbaImage` in the given format, for a round trip.
    fn encode(img: &image::RgbaImage, format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img.clone())
            .write_to(&mut Cursor::new(&mut bytes), format)
            .expect("encoding");
        bytes
    }

    fn sample() -> image::RgbaImage {
        // 4x3, with a red top-left corner and a green bottom-right one.
        let mut img = image::RgbaImage::from_pixel(4, 3, image::Rgba([0, 0, 0, 255]));
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(3, 2, image::Rgba([0, 255, 0, 255]));
        img
    }

    #[test]
    fn png_round_trips_pixels_exactly() {
        let png = encode(&sample(), image::ImageFormat::Png);
        let data = decode(&png).expect("PNG decoding");
        assert_eq!((data.width(), data.height()), (4, 3));
        let rgba = data.rgba();
        // Pixel (0,0).
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
        // Pixel (3,2), the last one: offset (2*4 + 3) * 4.
        let last = ((2 * 4 + 3) * 4) as usize;
        assert_eq!(&rgba[last..last + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn jpeg_decodes_with_correct_dimensions() {
        // JPEG is lossy, so we only check the detected format and the dimensions.
        let jpeg = encode(&sample(), image::ImageFormat::Jpeg);
        let data = decode(&jpeg).expect("JPEG decoding");
        assert_eq!((data.width(), data.height()), (4, 3));
        assert_eq!(data.rgba().len(), 4 * 3 * 4);
    }

    #[test]
    fn format_is_detected_from_magic_bytes() {
        // No format hint is given: detection works from the bytes alone.
        let png = encode(&sample(), image::ImageFormat::Png);
        assert!(png.starts_with(&[0x89, 0x50, 0x4E, 0x47]), "PNG header");
        assert!(decode(&png).is_ok());
    }

    /// A photograph-like test card: a gradient, so a decoder that got the scale or the
    /// order of the pixels wrong shows it.
    fn card(width: u32, height: u32) -> image::RgbaImage {
        image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([(x * 255 / width) as u8, (y * 255 / height) as u8, 128, 255])
        })
    }

    #[test]
    fn a_full_size_decode_at_is_the_plain_decode() {
        let png = encode(&sample(), image::ImageFormat::Png);
        let data = decode_at(&png, DecodeSize::FULL).expect("PNG decoding");
        assert_eq!(data.rgba(), decode(&png).unwrap().rgba());
    }

    #[test]
    fn a_png_is_downsampled_to_the_size_asked() {
        let png = encode(&card(400, 300), image::ImageFormat::Png);
        let data = decode_at(&png, DecodeSize::new(Some(80), Some(80))).unwrap();
        // Covering an 80x80 box at the image's 4:3 ratio.
        assert_eq!((data.width(), data.height()), (107, 80));
        assert_eq!(data.rgba().len(), 107 * 80 * 4);
        // The gradient survives: red grows to the right.
        let red = |x: usize| data.rgba()[(40 * 107 + x) * 4];
        assert!(
            red(5) < red(50) && red(50) < red(100),
            "{} {} {}",
            red(5),
            red(50),
            red(100)
        );
    }

    #[test]
    fn a_jpeg_is_decoded_scaled_to_the_size_asked() {
        let jpeg = encode(&card(1600, 1200), image::ImageFormat::Jpeg);
        let data = decode_at(&jpeg, DecodeSize::new(Some(150), None)).unwrap();
        assert_eq!((data.width(), data.height()), (150, 113));
        // The middle pixel is the middle of the gradient, whatever the scale took.
        let mid = &data.rgba()[(56 * 150 + 75) * 4..][..4];
        assert!((i32::from(mid[0]) - 127).abs() < 12, "{mid:?}");
        assert!((i32::from(mid[1]) - 127).abs() < 12, "{mid:?}");
        assert_eq!(mid[3], 255);
    }

    #[test]
    fn the_scaled_jpeg_path_agrees_with_the_full_one() {
        let jpeg = encode(&card(800, 600), image::ImageFormat::Jpeg);
        let size = DecodeSize::new(Some(100), Some(75));
        let scaled = scaled_jpeg(&jpeg, size).expect("the scaled path takes this JPEG");
        let full = image::load_from_memory(&jpeg).unwrap().to_rgba8();
        let reference = downsample(full, 100, 75);
        let worst = scaled
            .rgba()
            .iter()
            .zip(reference.as_raw())
            .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
            .max()
            .unwrap();
        assert!(worst <= 16, "the two paths differ by {worst}");
    }

    #[test]
    fn a_halving_is_the_mean_of_four() {
        let img = image::RgbaImage::from_fn(5, 3, |x, y| {
            image::Rgba([(x * 40) as u8, (y * 100) as u8, 7, 255])
        });
        let half = halve(&img);
        assert_eq!(half.dimensions(), (2, 1));
        // Columns 2 and 3, rows 0 and 1: red (80+120)/2, green (0+100)/2.
        assert_eq!(half.get_pixel(1, 0).0, [100, 50, 7, 255]);
    }

    #[test]
    fn a_large_reduction_matches_the_filter_alone() {
        let img = card(1200, 900);
        let ours = downsample(img.clone(), 75, 56);
        let filter = image::imageops::resize(&img, 75, 56, image::imageops::FilterType::Triangle);
        let worst = ours
            .as_raw()
            .iter()
            .zip(filter.as_raw())
            .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
            .max()
            .unwrap();
        assert!(worst <= 6, "the halvings moved a pixel by {worst}");
    }

    #[test]
    fn nothing_is_decoded_larger_than_the_file() {
        let png = encode(&sample(), image::ImageFormat::Png);
        let data = decode_at(&png, DecodeSize::new(Some(400), Some(300))).unwrap();
        assert_eq!((data.width(), data.height()), (4, 3));
    }

    #[test]
    fn transparent_pixels_do_not_darken_a_downsampled_edge() {
        // White, half of it fully transparent *black*: averaged unpremultiplied, the
        // visible half's edge turns grey.
        let img = image::RgbaImage::from_fn(64, 64, |x, _| match x < 32 {
            true => image::Rgba([255, 255, 255, 255]),
            false => image::Rgba([0, 0, 0, 0]),
        });
        let png = encode(&img, image::ImageFormat::Png);
        let data = decode_at(&png, DecodeSize::new(Some(8), None)).unwrap();
        // Column 3 of 8 straddles the edge (source columns 24..32 and the filter's reach
        // past them): partly transparent, and still white.
        let p = &data.rgba()[(4 * 8 + 3) * 4..][..4];
        assert!(p[3] > 0 && p[3] < 255, "{p:?}");
        assert!(p[0] >= 250, "a darkened edge: {p:?}");
    }

    #[test]
    fn dimensions_come_from_the_header() {
        let png = encode(&card(64, 48), image::ImageFormat::Png);
        assert_eq!(dimensions(&png), Some((64, 48)));
        let jpeg = encode(&card(64, 48), image::ImageFormat::Jpeg);
        assert_eq!(dimensions(&jpeg), Some((64, 48)));
        assert_eq!(dimensions(b"not an image"), None);
    }

    #[test]
    fn garbage_bytes_error_cleanly() {
        let err = decode(b"not an image at all").unwrap_err();
        assert!(!err.message().is_empty());
        let size = DecodeSize::new(Some(10), None);
        assert!(decode_at(b"not an image at all", size).is_err());
        assert!(decode_at(&[0xFF, 0xD8, 0, 1, 2], size).is_err());
    }
}
