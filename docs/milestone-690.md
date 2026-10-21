# Milestone 690 — SVG pictures

## Objective

#303: applications need vector artwork — icons, illustrations, logos, empty states — and
frus could show bitmaps and its own icon set only. An SVG drawn as a bitmap blurs when it is
scaled and costs a texture per size; drawn as paths it is sharp at any size and on any screen.

## What was done

- **`frus-svg`**, a new crate. `Svg::parse` reads an SVG file with usvg (default features off:
  no text shaping, no system fonts), which resolves CSS, `<use>`, units, `objectBoundingBox`
  gradients and the rest, and turns the tree into shapes in the picture's own coordinates:
  - paths with their fill rule;
  - **strokes expanded into outlines** — dashed first, then widened with their caps, joins and
    miter limit by `tiny_skia_path` (already usvg's) — and filled non-zero; an outline is what
    lets a stroke take a gradient, and it needed nothing from the renderer;
  - flat colours, and linear and radial gradients as `GradientRamp`s (milestone 689) with
    their stops, spread method and `gradientTransform`, composed with the shape's transform;
  - group opacity and clip paths as layers (a clip path's shapes united into one outline);
  - masks, filters, patterns, embedded bitmaps, text and blend modes are not drawn yet, and
    `Svg::unsupported` names the ones a picture uses.
- **`Svg::paint`** maps the shapes into a box with a `BoxFit` and an alignment (a cropped
  picture is clipped to its box), at an opacity, and can draw every shape in one colour,
  keeping each shape's transparency — a single-colour icon recoloured to the theme.
- **`SvgPicture`** (`frus-widgets`, the new `svg` feature, off by default): `memory` (embedded
  bytes, parsed once per process, keyed by address as `Image::memory` is), `string` (parsed
  once per text), `new` (a parsed `SvgHandle`); sized like `Image` (the picture's `width` and
  `height` unless told), `fit`, `alignment`, `color`, `opacity`, `semantic_label`,
  `exclude_from_semantics`, `error`. The console says once when a picture cannot be parsed or
  uses something not drawn.

## A difference that is the framework's

Group opacity is one layer, as SVG says, but frus composites translucency in linear light
where SVG renderers do it in sRGB, so a half-faded group comes out paler than in a browser
(milestone 437 and the roadmap). It is pinned by its own test rather than hidden in a
tolerance.

## Not done yet (rest of #303)

- `SvgPicture::network`.
- Masks, filters, patterns, embedded bitmaps, text.
- Paths are tessellated again every frame, as every path here is: a picture of thousands of
  shapes wants a cache.
- Measured on Android and the Web.

## Verified

`frus-svg` unit tests: a shape placed in its box, transforms reaching the points, a stroke as
a non-zero outline reaching its caps, dashes, a user-space gradient and an
`objectBoundingBox` radial one landing on the screen, opacity and clip paths as layers, the
tint, a cropped picture staying in its box, `unsupported`. `SvgPicture`: its sizes, parsing
once, painting recoloured, a broken source. On the GPU (`frus-test/tests/svg.rs`), four
pictures drawn by frus and by resvg — shapes and fill rules; strokes with round, miter and
bevel joins and dashes; three gradients with stops, a focal point, `reflect` and a rotation;
transforms and a clip path — agree on all but at most 2 % of pixels (edges); dropping the
gradient transform on purpose made 11 053 pixels disagree.
