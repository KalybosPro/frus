# Milestone 743 — Images drawn small are averaged

## Objective

Part of #221, left by milestones 687 and 688: an image drawn much smaller than its texture
shimmered. One screen pixel covered many of the image's, a bilinear sample read four of them,
and what came out depended on where the samples landed: a fine pattern turned solid or striped,
thin lines vanished or filled the picture, and the picture crawled while its scale animated.
Decoding at the shown size (687) avoids most of it, but not for an image given no size, a
picture that scales, or one shown at several sizes at once.

## What was done

- **`FilterQuality`** (`frus-core`): `None` (nearest), `Low` (bilinear on the image itself),
  `Medium` (bilinear between pre-shrunk copies, the default) and `High`, which filters as
  `Medium` when shrinking and as `Low` when enlarging; frus has no bicubic filter yet. It
  rides on `Primitive::Image` as `filter_quality`. `Image::filter_quality`,
  `DecorationImage::filter_quality` and `Scene::draw_image_with` choose it; `draw_image`,
  `draw_image_strict` and `Scene::image` keep the default.
- **Mip chains** (`frus-gpu`, `mipmap.rs` and `mipmap.wgsl`): the first time an image is drawn
  smaller than its pixels at `Medium` or `High`, its texture is made again with every level and
  the levels past the first are rendered on the GPU, one pass each. Each texel of a level is the
  box average of the texels it covers in the level above: pairs for an even span, three texels
  with the exact weights `(d - x, d, x + 1) / (2d + 1)` for an odd one, so no row or column is
  dropped. The loads decode sRGB and the sRGB target encodes, so the average is taken in linear
  light; colours are weighted by alpha, so a transparent texel's colour does not tint the edge
  of what is next to it. An sRGB render target and `textureLoad` are core in WebGL2 as in every
  native backend.
- **Three samplers**: nearest and bilinear, both clamped to the first level, and trilinear. A
  texture that a `Medium` draw gave a chain still reads as before through `Low` or `None`.
- **Memory**: only an image drawn smaller than its pixels pays for a chain, a third more than
  its texture. Any shrinking counts, not only below one half: between one and one half the
  sampler already blends in the second level, and a chain made later would change the picture
  the frame it appeared. An image only ever shown at its size or larger keeps one level.
- **Eviction once a frame**: the image painter evicted, at every `prepare_frame`, the textures
  that call had not used. A frame with layers prepares several times, so a layer's images and
  the frame's evicted each other and were uploaded again each time a layer was rendered. Eviction now happens in
  `ImagePainter::end_frame`, after the frame's last prepare, as the path painter's does — a
  chain is not remade every frame.
- **Tests**: `mipmap.rs` checks the level count and sizes, that a chain costs a third more at
  most, that the odd-span weights count every texel equally, and when a draw needs a chain.
  `frus-test`'s `mipmaps.rs` draws a 256×256 one-pixel checkerboard at one eighth, a sixteenth
  of a pixel off the grid so that a first-level sample is exactly one texel: at `Medium` and
  `High` every pixel is the grey of linear 0.5 (sRGB 188) within a few levels, while `Low` and
  `None` alias to black or white. Lines one column in eight average to linear 1/8; a red and
  transparent-green checkerboard averages to red without green; an image at its own size, drawn
  beside a shrunk copy that remakes its texture mid-frame, keeps its exact pixels at every
  quality.

## Not done

- **Transformed layers** have no chain: a layer scaled below one half by a `Transform` still
  skips texels (milestone 732's note).
- **No bicubic filter**: `High` enlarges as `Low` does.
- **A strict draw** (a nine-slice piece) keeps its sampling inside its region on the first
  level; drawn far smaller than its pixels, a coarse level can reach a texel or two past it.
- **Images in a texture atlas**: none share one today, so the chain is per image.
- Not run on a phone. The passes are plain render passes in an sRGB format, which the Mali-G51
  and WebGL2 support; the tests ran on the Windows laptop's GPU.
