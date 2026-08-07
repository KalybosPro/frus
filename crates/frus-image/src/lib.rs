//! `frus-image` — image **decoding** (PNG/JPEG) into [`frus_core::ImageData`].
//!
//! A thin, **optional** layer: `frus-core` is dependency-free and holds nothing but
//! raw pixels; this crate adds the decoder dependency (`image`) to turn file bytes
//! into an `ImageData` ready to upload. Applications use it to load their assets
//! (`decode(include_bytes!("logo.png"))`) without `frus-widgets` or `frus-core`
//! inheriting the decoder's dependency tree.

use std::error::Error;
use std::fmt;

use frus_core::ImageData;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Encodes a test `RgbaImage` in the given format, for a round trip.
    fn encode(img: &image::RgbaImage, format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img.clone())
            .write_to(&mut Cursor::new(&mut bytes), format)
            .expect("encodage");
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

    #[test]
    fn garbage_bytes_error_cleanly() {
        let err = decode(b"not an image at all").unwrap_err();
        assert!(!err.message().is_empty());
    }
}
