# frus-image

PNG and JPEG decoding into frus's `ImageData`.

**Layer:** Foundations. A deliberately narrow crate: `frus-core` holds raw pixels and no
decoder, and this one adds [`image`](https://crates.io/crates/image), with only the PNG
and JPEG formats turned on, so nothing else inherits its dependency tree. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `decode`: bytes in, sRGB RGBA `ImageData` out, the format detected from the bytes.
- `DecodeError`: an unrecognised format or corrupt data.

## Example

```rust
// Encode a 2x2 red image as PNG, then decode it back.
let red = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
let mut png = Vec::new();
image::DynamicImage::ImageRgba8(red)
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .unwrap();

let decoded = frus_image::decode(&png).unwrap();
assert_eq!((decoded.width(), decoded.height()), (2, 2));
```

## Part of frus

Applications use it through `frus-widgets`' `images` feature (`Image::memory`, `asset!`),
on by default. See the [workspace README](https://github.com/KalybosPro/frus#readme).
Licensed under MIT or Apache-2.0.
