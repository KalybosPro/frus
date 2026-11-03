# Milestone 737 — Masks and filters in SVG pictures

## Objective

#303 left masks, filters and patterns out of `frus-svg`: each was skipped and named by
`Svg::unsupported`. Exported artwork leans on two of them more than on anything else still
missing. A fade or a cut-out is a `<mask>`, and nearly every card, button or icon a design tool
exports with a shadow carries a `<filter>`: the same few primitives, offset, blur, a composite
and colour matrices. Drawn without them, a masked picture shows what it should hide and a
shadowed one loses its shadow.

## What was done

Neither needed anything new in the renderer. Each is built from the scene's layers, which can
already blur, filter colours and meet what is under them by a blend mode (milestones 339 and
646). The picture's items gained an `Effect` on their groups (a blur, a colour filter, a blend,
and whether the tint reaches them), and the new `effect.rs` builds those groups.

- **Masks.** The masked group becomes a layer of its own, and the mask is a last layer inside
  it that meets the group's pixels with `DstIn`, so the group is kept where the mask is opaque:
  - **alpha**: that layer holds the mask's content, clipped to the mask's region;
  - **luminance**, the default: the content again inside it, its luminance turned into alpha by
    a colour matrix (the reference renderer's weights, on sRGB values), meeting the first with
    `DstIn` too. That makes the coverage luminance times alpha. A matrix alone could not do it,
    because it is linear in the straight colour and the coverage is a product;
  - a mask with nothing in it hides its group; a masked mask masks the group again, as the
    reference renderer does; the mask layer is kept even when it draws nothing, and a tint never
    reaches the mask's content, which would otherwise change what it keeps.
- **Filters** are evaluated as a graph of item lists, one per named result, with `SourceGraphic`,
  `SourceAlpha` (the source in black through an alpha-only matrix) and references:
  - `feGaussianBlur`: a blurred layer, the deviations taken through the group's scale and, when
    painted, the picture's. A blur longer one way than the other on a turned or slanted group
    would be blurred along the screen's axes, so it is not drawn;
  - `feOffset`: the items moved; `feFlood`: a rectangle filling the subregion; `feMerge`: the
    inputs listed in order;
  - `feDropShadow`: the input's silhouette in the shadow colour (`ColorFilter::Mode` with
    `SrcIn`), blurred and moved, with the input over it;
  - `feComposite`: `over` lists the two; `in` and `out` are a layer holding the second input
    and the first meeting it by `SrcIn` or `SrcOut`. `atop`, `xor` and `lighter` mix two colours,
    so they are drawn only when the filter asks for sRGB; `arithmetic` is not drawn;
  - `feBlend` in its normal mode, which is `over`;
  - `feColorMatrix`: frus's matrix works on sRGB values and SVG's filters default to linear
    light. In sRGB any matrix is drawn. In linear light, only one whose colour rows each keep
    their channel or set a constant (re-encoded as sRGB) and whose alpha does not depend on the
    colour. Those are the matrices design tools export to make a shadow's alpha and tint it.
    Saturation, hue rotation and luminance-to-alpha are drawn in sRGB only;
  - a primitive's subregion clips its result, and the filter region clips the whole;
  - a filter holding any other primitive (lighting, turbulence, convolution, displacement,
    morphology, component transfer, tile, image) is left off whole: the group is drawn unfiltered
    and `"filter"` is named, as before.
- `Svg::unsupported` no longer names `"mask"`; it names `"filter"` only for those filters.

## Considered

- **A mask primitive in the scene**, a layer whose coverage is another list of primitives.
  It would be the most direct design, but it means a new renderer path on every backend, and
  the blend modes already give the same pixels.
- **Morphology as `ImageFilter::Dilate`/`Erode`**: SVG's kernel is a rectangle and frus's
  reach was not checked to be one. It was left out rather than drawn slightly wrong.
- **Patterns**: a pattern fill needs its tile drawn once and repeated under the shape, which
  the scene has no primitive for. They stay unsupported.

## A difference that is the framework's

Inside a mask or a filter, translucent shapes meet in linear light, as everywhere in frus. For
filters that is what SVG asks by default (`color-interpolation-filters: linearRGB`). For a
mask's own content (two translucent shapes overlapping inside it), the reference renderer mixes
in sRGB, so the luminance it keeps can differ there. Opaque mask content, the usual case,
agrees.

## Tests

- `frus-svg` unit tests: an alpha mask is a `DstIn` layer, unclipped itself, with its content
  clipped to the mask's region; a luminance mask adds the luminance layer; an empty mask hides
  its group and a tint spares the mask; a blur is a blurred layer whose deviations follow the
  group's and the picture's scales, clipped to the default filter region; a drop shadow is the
  coloured, blurred, moved silhouette under the source; a design tool's shadow graph (flood,
  alpha matrix, offset, blur, `out`, tint matrix, blend, merge) is drawn, its linear-light
  constant re-encoded; a saturation matrix is drawn in sRGB and not in linear light; an
  anisotropic blur on a turned group, turbulence and a pattern are named.
- On the GPU (`frus-test/tests/svg.rs`), three pictures compared with resvg, the reference laid
  on white in linear light as frus lays a layer: masks (an alpha mask, a luminance gradient mask
  in `objectBoundingBox` content units straight and turned, a ring through a mask region on a
  turned group), blurs (uniform, anisotropic with a wide region, a drop shadow, a blur on a scaled
  group) and the design tool's shadow graph on a translucent card. Each agrees on all but 2% of
  pixels. On purpose, an alpha mask that keeps nothing made 3 350 pixels disagree, a blur at 0.3
  of its strength 2 624, and `out` drawn as `over` 4 544.
- `cargo test -p frus-test --tests` under WSL: no golden moved.

## Not done

- Patterns, text, and the filter primitives listed above.
- Blend modes other than normal, on groups (`mix-blend-mode`) and in `feBlend`.
- Checked on a desktop GPU and on WSL's software renderer only, not on Android or in a
  browser. A blurred layer takes the whole target, so a picture with many filtered groups costs a
  full-size pass for each; this was not measured on the test phone.
