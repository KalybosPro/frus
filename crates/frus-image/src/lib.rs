//! `frus-image` — **décodage** d'images (PNG/JPEG) vers [`frus_core::ImageData`].
//!
//! Couche fine et **optionnelle** : `frus-core` (zéro-dép) ne détient que des
//! pixels bruts ; ce crate ajoute la dépendance au décodeur (`image`) pour
//! transformer des octets de fichier en `ImageData` prêt à téléverser. Les
//! applications l'utilisent pour charger leurs ressources
//! (`decode(include_bytes!("logo.png"))`) sans que `frus-widgets`/`frus-core`
//! n'héritent de l'arbre de dépendances du décodeur.

use std::error::Error;
use std::fmt;

use frus_core::ImageData;

/// Échec de décodage : format non reconnu ou données corrompues.
#[derive(Debug)]
pub struct DecodeError(String);

impl DecodeError {
    /// Le message d'erreur sous-jacent.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "échec de décodage d'image : {}", self.0)
    }
}

impl Error for DecodeError {}

/// Décode des octets d'image (PNG ou JPEG — **format détecté** aux octets
/// magiques) en [`ImageData`] RGBA sRGB. Toute image est convertie en RGBA8.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Encode une image 2×2 en PNG, puis décode-la.
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

    /// Encode une `RgbaImage` de test au format donné (pour un aller-retour).
    fn encode(img: &image::RgbaImage, format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img.clone())
            .write_to(&mut Cursor::new(&mut bytes), format)
            .expect("encodage");
        bytes
    }

    fn sample() -> image::RgbaImage {
        // 4×3, coin haut-gauche rouge, coin bas-droit vert.
        let mut img = image::RgbaImage::from_pixel(4, 3, image::Rgba([0, 0, 0, 255]));
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(3, 2, image::Rgba([0, 255, 0, 255]));
        img
    }

    #[test]
    fn png_round_trips_pixels_exactly() {
        let png = encode(&sample(), image::ImageFormat::Png);
        let data = decode(&png).expect("décodage PNG");
        assert_eq!((data.width(), data.height()), (4, 3));
        let rgba = data.rgba();
        // Pixel (0,0).
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
        // Pixel (3,2) = dernier : offset (2*4 + 3) * 4.
        let last = ((2 * 4 + 3) * 4) as usize;
        assert_eq!(&rgba[last..last + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn jpeg_decodes_with_correct_dimensions() {
        // JPEG est avec perte : on ne vérifie que le format détecté + les dims.
        let jpeg = encode(&sample(), image::ImageFormat::Jpeg);
        let data = decode(&jpeg).expect("décodage JPEG");
        assert_eq!((data.width(), data.height()), (4, 3));
        assert_eq!(data.rgba().len(), 4 * 3 * 4);
    }

    #[test]
    fn format_is_detected_from_magic_bytes() {
        // Pas d'indice de format fourni : la détection se fait aux octets.
        let png = encode(&sample(), image::ImageFormat::Png);
        assert!(png.starts_with(&[0x89, 0x50, 0x4E, 0x47]), "en-tête PNG");
        assert!(decode(&png).is_ok());
    }

    #[test]
    fn garbage_bytes_error_cleanly() {
        let err = decode(b"not an image at all").unwrap_err();
        assert!(!err.message().is_empty());
    }
}
