# Milestone 645 — Pictures as backgrounds, and blur styles

## Objective

The third of the `BoxDecoration` milestones, after 641 (shape, sides, shadows) and 644
(gradients). Two things the reference's decoration has were still missing:

| | reference | frus before |
|---|---|---|
| `DecorationImage` | a picture under the border, fitted, aligned, sliced, repeated, mirrored, filtered (`painting/decoration_image.dart`) | none: a picture behind content needed a `Stack` |
| `paintImage`, `applyBoxFit` | the drawing behind it: a fit that would overflow crops the picture instead | `BoxFit::apply`, which lets `None`, `FitWidth` and `FitHeight` spill past the box |
| `ImageRepeat` | `repeat`, `repeatX`, `repeatY`, `noRepeat` | none |
| `BoxShadow.blurStyle` | `normal`, `solid`, `outer`, `inner` (`painting/box_shadow.dart`) | a shadow was always blurred both ways |

## What was done

- **`frus_core::DecorationImage`**, with the reference's fields and defaults:
  - `fit` (none named: `ScaleDown`, or `Fill` with a centre slice);
  - `alignment`, resolved by the box's reading direction;
  - `center_slice`, `repeat`, `match_text_direction`;
  - `scale` (pixels per logical pixel), `opacity`, `color_filter`, `invert_colors`.

  The picture is decoded pixels, `Option<ImageHandle>`. One still on its way is `None` and
  paints nothing, as the reference's does until it arrives.
- **`DecorationImage::pieces`** is the reference's `paintImage` short of the drawing: where
  each part of the picture lands and which part it is.
  - The fit is the reference's `applyBoxFit` (`frus_core::apply_box_fit`, `FittedSizes`).
  - The repeated copies are its `_generateImageTileRects`.
  - A centre slice is drawn in nine parts, as `drawImageNine` draws it: corners at their
    own size, edges stretched along their length, the middle both ways.
  - A mirror reflects every part about the box's centre and reads it backwards.

  A pattern repeated over a large box stops at `MAX_IMAGE_TILES` (10 000) copies, where the
  reference has no limit.
- **`DecorationImage::paint`** draws the parts:
  - repeated ones clipped to the box;
  - the colour filter, then the inversion, each as a layer;
  - `lerp` crosses two pictures over, and a picture on one side only fades.
- **`BoxDecoration::image` and `Container::image`.**
  - The picture is painted into the whole box, between the background and the border
    (`box_decoration.dart:476`).
  - It is clipped to the corners or the circle. A box partly clipped away is clipped by its
    outline, because a shape inscribed in what is left of the box would be the wrong shape.
  - A uniform border, otherwise drawn with the fill as one primitive, is drawn over the
    picture.
- **Strict sampling**: `Primitive::Image::strict`, `Scene::draw_image_strict`.
  - A slice's parts are stretched apart from each other, and filtering at a part's edge
    reached into its neighbour: a frame's white bled across its blue middle. The renderer
    now clamps sampling to the part drawn, inset by half a texel, when asked.
  - The reference does the same for slices and not for a plain draw, so a cropped picture
    is sampled as before. Clamping to the whole texture, which is what a plain draw does, is
    the edge clamp it always had.
- **`BlurStyle` and `BoxShadow::blur_style`.**
  - The rectangle shader keeps both sides of the blurred edge, the inside solid, the outside
    only, or the inside only, against the shape's crisp edge `blur` inside the quad.
  - The style rides in the slot a flat rectangle leaves free.
  - Lerp follows the reference: a plain blur takes the other's style.
  - `Scene::styled_shadow` paints one.

**Changed**: `Primitive::Rect` has `blur_style`, `Primitive::Image` has `strict`,
`BoxShadow` has `blur_style` and `BoxDecoration` has `image`.

## Decisions

- **The cross-over is drawn as the leaving picture whole and the arriving one over it at
  `t`, in a group.**
  - The reference adds the two at `1 - t` and `t` (`BlendMode.plus`), which the renderer
    cannot do for one primitive.
  - Where the arriving picture is opaque the two are the same.
  - Where it is translucent, the leaving one shows a little more than in the reference. The
    blend modes are milestone 646's.
- **A slice in a box too small for its corners keeps them at their size, centred, past the
  box.** That is the reference's arithmetic: the fitted middle is nothing, and the corners
  are added back. The decoration's clip trims it when the box is rounded.

## Not done here

- `filterQuality`, `isAntiAlias` and `onError`. The renderer has one sampler, and a
  failed picture is reported by `Image::error`.
- `backgroundBlendMode`: milestone 646.
- The `Image` widget's own `repeat` and `centerSlice` (tracker): `pieces` already lays them
  out.
- A widget giving a subtree its own reading direction, the reference's `Directionality`
  (tracker). The direction comes from the theme.

## Tests

- `frus-core`, the fitting and the layout:
  - the reference's fit table, and empty sizes fitting nothing;
  - a small picture centred, and aligned;
  - a cover keeping the aligned part;
  - repeats along each axis, from where the picture sits, clipped to the box, stopping at
    the tile limit, and none when the picture fills the box;
  - a centre slice: corners at their size, edges and middle stretched, every part strict;
    corners in a box too small; a repeated slice; a slice at a scale;
  - a picture at a scale;
  - mirroring in a right-to-left box: one picture, and a slice whose sides swap; a start
    alignment resolving to the right.
- `frus-core`, painting and the decoration:
  - painting: opacity times the blend; the filter before the inversion; repeats clipped to
    the box and the clip restored; two pictures crossing over, and one alone fading;
  - the decoration: the picture between the fill and the border; the corners' and the
    circle's clips into the whole box; the outline when partly clipped; a picture fading in;
  - blur styles painted, and their lerp.
- `frus-test`: a new picture, `decoration_images`, checked by eye:
  - top: a cover keeping its top, a repeat, a frame stretched round its slice, a picture in a
    bordered circle;
  - middle: a glow in each of the four blur styles;
  - bottom: a picture turned grey, inverted, at half its opacity, and at twice its size in a
    corner.

  The other 102 pictures are unchanged.

Mutation testing: 24 mutants, all killed, the three in the renderer by the picture. They
cover:

- each fit that crops, and the scale-down;
- a picture filling its box not repeating;
- the mirrored alignment, the mirror itself, and the crop's alignment;
- the last tile and the tile limit;
- a slice's corners and border at a scale, and its strictness;
- a repeat's clip, and the inversion;
- the cross-over's leaving picture, its group, and the settings it reports;
- the fill giving up its line to the picture, and the outline clip;
- a shadow's style painted and lerped;
- the outer style in the shader, the style's slot, and the strict region.
