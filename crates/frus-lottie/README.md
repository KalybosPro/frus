# frus-lottie

Lottie animations drawn as frus's own vector paths.

**Layer:** Foundations. `Animation::parse` reads a Lottie file (Bodymovin JSON) with [velato](https://crates.io/crates/velato), and `Animation::paint` draws any moment of it into a `Scene` at any size, as paths the renderer tessellates at the size shown.

Drawn: shapes and their animated properties, fills, strokes (widths, caps, joins, dashes), flat colours and linear and radial gradients, layer opacity, clips and masks, the blend modes of mattes, image layers (a PNG or JPEG embedded in the file, or given by the application with `Animation::with_image`). Not yet: sweep gradients, named by `Animation::unsupported`.

Applications use it through the `Lottie` widget in `frus`, with the `lottie` feature.

## Part of frus

See the [frus repository](https://github.com/KalybosPro/frus).
