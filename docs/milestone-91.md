# Milestone 91 — Image decoding (PNG/JPEG)

## Analysis

Milestone 90 laid down GPU texture management, but could only start from **raw
pixels** (`ImageData::from_rgba`). A real app loads its images from **files**
(`logo.png`, `photo.jpg`). So the decoder was missing — the thin layer that turns
file bytes into an `ImageData`.

Milestone 90's architectural choice was deliberate: `frus-core` stays
**zero-dependency** (it only holds pixels). The decoder (the `image` crate, with
its dependencies: `png`, `jpeg-decoder`…) is isolated in an **optional** crate, so
that `frus-core`/`frus-widgets` do not inherit it.

## Architecture

```
             bytes (PNG/JPEG)
                   │
   frus-image::decode  (the `image` crate, png+jpeg formats)
                   │  format detection, → RGBA8
                   ▼
   frus_core::ImageData ──► (milestone 90) a cached GPU texture
```

A new crate **`frus-image`** (depending on `frus-core` + `image`), at the same
level as `frus-text` in the dependency graph. A single public function:

```rust
pub fn decode(bytes: &[u8]) -> Result<ImageData, DecodeError>;
```

- **Format detected from the magic bytes** (no extension required).
- Every image is converted to **RGBA8 sRGB** (the format the renderer expects).
- `DecodeError` hides the `image` crate's error type (a stable, decoupled API).

The `image` crate is configured `default-features = false, features =
["png", "jpeg"]` to **keep the dependency tree down** to the two target formats.

## Technical decisions

- **A separate crate rather than putting it in `frus-core`/`frus-widgets`.** The
  decoder is heavy (formats, zlib…) and is not needed everywhere: an app doing
  only procedural drawing should not pay for it. Apps that load assets depend on
  `frus-image` explicitly. `frus-core` keeps its zero-dependency invariant.
- **`decode(bytes)` alone**, with no file reading and no networking: the *what*
  (bytes) is supplied by the app (`include_bytes!`, `std::fs::read`, a
  download…). The crate does one thing. An `ImageProvider` (asset / network /
  memory) can sit on top later.
- **A demo asset with reproducible provenance.** The committed PNG is not an
  opaque binary: the `frus-image/examples/gen_logo.rs` example regenerates it
  (`cargo run -p frus-image --example gen_logo > crates/frus-demo/assets/logo.png`).

## Tests

- `png_round_trips_pixels_exactly`: encodes a known 4×3 image → decodes → **exact**
  dimensions and pixels (red/green corners).
- `jpeg_decodes_with_correct_dimensions`: a (lossy) JPEG → correct dimensions and
  buffer size (format detected).
- `format_is_detected_from_magic_bytes`: a PNG header recognised with no hint.
- `garbage_bytes_error_cleanly`: invalid bytes → `Err` with a message.
- A doctest: an encode→decode round trip of a 2×2.

## Demo

`demo_image()` now loads a **decoded bundled PNG**
(`decode(include_bytes!("../assets/logo.png"))`), instead of milestone 90's
generated gradient (kept as a **fallback** if decoding fails). The image is
decoded once (`OnceLock`) and then cached by identity on the renderer side.

## What's left

- Other formats (WebP, GIF, …): add features from the `image` crate.
- An `ImageProvider`: an asset/network/memory abstraction + **asynchronous**
  loading (through `Command`) so as not to decode on the UI thread.
- Mipmaps (milestone 90) for a clean downscale of large photos.
