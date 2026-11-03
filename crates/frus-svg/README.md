# frus-svg

SVG pictures drawn as frus's own vector paths.

**Layer:** Foundations. `Svg::parse` reads an SVG file with [usvg](https://crates.io/crates/usvg) into shapes in the picture's coordinates; `Svg::paint` draws them into a `Scene` at any size, as paths the renderer tessellates at the size shown, so a picture stays sharp at any zoom.

Drawn: paths and fill rules; strokes with width, caps, joins, miter limit and dashes; flat colours and linear and radial gradients (any number of stops, spread method, transform); group opacity; clip paths; masks, luminance and alpha; filters made of Gaussian blurs, drop shadows, offsets, floods, merges, `over`/`in`/`out` composites, normal blends and the colour matrices that mean the same in sRGB as in linear light; embedded PNG and JPEG bitmaps and SVG pictures (from `data:` addresses only: a picture never opens a file it names). Not yet: patterns, the other filter primitives (lighting, turbulence, convolution, displacement, morphology, component transfer, arithmetic compositing, blend modes other than normal), GIF and WebP bitmaps and text, each named by `Svg::unsupported`; a filter using one is drawn unfiltered.

Applications use it through `SvgPicture` in `frus`, with the `svg` feature.

## Part of frus

See the [frus repository](https://github.com/KalybosPro/frus).
