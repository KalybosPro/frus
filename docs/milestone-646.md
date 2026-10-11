# Milestone 646 — Blend modes, and backgrounds blended with what is under them

## Objective

The last of the `BoxDecoration` milestones (641, 644, 645): its `backgroundBlendMode`. The
reference paints the background — the colour or the gradient — with that blend mode, so it
combines with what is already painted under the box: a multiply that darkens a picture, a
colour that tints it, a difference that inverts it (`box_decoration.dart:413`).

frus could not do this anywhere. Its blend modes were the ones a colour filter or a mask
uses, inside a layer. Nothing blended with the frame under it, and the set was short of the
reference's by eleven:

| | reference | frus before |
|---|---|---|
| a paint's blend mode against what is under it | any of 29 | painting over only |
| `BlendMode` | 29 modes | 18: no `clear`, `colorDodge`, `colorBurn`, `hardLight`, `softLight`, `difference`, `exclusion`, `hue`, `saturation`, `color`, `luminosity` |
| `BoxDecoration.backgroundBlendMode` | yes | no |
| `DecorationImage.lerp` of two pictures | their sum (`BlendMode.plus`) | an approximation (milestone 645) |

## What was done

- **`LayerFilter::blend`**: how a layer meets what is under it. `None` paints it over, as
  every layer always has. `Scene::blended` and `Scene::blended_in` (in a clip shape) make
  one.
  - The layer's clip shape is its coverage: outside it, what was there stays. The opacity
    belongs to the layer's colour, as a paint's alpha does.
  - A blend folds with no other filter: it is how the whole layer meets what is under it,
    and a filter folded beside it would apply before or after it depending on which widget
    it came from. It folds with nothing, which is what a clip is.
- **The renderer** reads the frame painted so far, the way a backdrop does.
  - At a blended layer the frame is cut: drawn up to it, copied, and the layer composited
    against the copy, replacing the pixels under it rather than painting over them.
  - The composite shader blends the two premultiplied in sRGB, as colours are written and
    as the reference blends, and mixes the result into what was there by the clip's
    coverage.
  - Backdrops and blends share that machinery (`Cut`, `draw_segmented`). It runs **inside
    a group too**: a blend in a group meets the group's own drawing, and the group is
    composited as before. A decorated box is often inside one — a fading page, a clipped
    card — and a blend that only worked at the top of the frame would quietly paint over
    there.
- **`BlendMode`** has the reference's whole set. The new eleven follow the W3C compositing
  formulas the reference's engine uses, and colour filters and masks have them too.
- **`BoxDecoration::background_blend_mode` and `Container::background_blend_mode`.**
  - The background is painted square, in a blended group shaped like the box: its circle,
    its rounded corners, or its outline when the box is partly clipped away.
  - The border is drawn after it, plainly; the shadows and the picture are not blended.
  - With no colour or gradient there is nothing to blend, and painting over is no group at
    all.
- **Two pictures cross over by their sum**, the reference's: the leaving one at `1 - t`, the
  arriving one added at `t`, in a group of their own.

**Changed**: `LayerFilter` has `blend`, `BoxDecoration` has `background_blend_mode`.

## Decisions

- **A blend mode changes half way through a transition**, as the shape does. The reference's
  `BoxDecoration.lerp` leaves it out, so a blended box painted plainly through every
  transition and blended again at the end.
- **A layer with a backdrop is painted over it**, not blended: the frame is cut once at a
  layer. The two are not combined anywhere in the framework.

## Not done here

- Blends with a backdrop on the same layer, as above.
- A backdrop inside a group: unchanged, as before this milestone.

## Tests

- `frus-core`:
  - a blend folds with nothing and with nothing else, in either direction; every mode has
    its own code;
  - the blended background: a group in the box's shape with the fill square inside it
    (rounded, circle, square, partly clipped); the border plain after it; painting over and
    an empty background not grouped; a gradient blended; the mode changing half way;
  - two pictures crossing over: the leaving one at `1 - t`, the arriving one added in a
    group.
- `frus-test`, `blend_modes`, read from the pixels:
  - every one of the 29 modes against its formula, worked on the CPU in sRGB, inside the
    clip, and the frame untouched outside it;
  - a translucent layer blending by its alpha;
  - a circle's coverage;
  - a blend inside a group meeting the group's drawing and not the frame's.
- `frus-test`: a new picture, `decoration_blend_modes`, checked by eye:
  - top: multiply, screen, overlay and difference, in rounded boxes over a picture;
  - middle: colour, luminosity, hue and saturation, in circles;
  - bottom: a multiply inside a group at 70 % opacity, an exclusion and a colour burn over
    a gradient, and two pictures half-way through crossing over.

  The other 103 pictures are unchanged, the backdrops' among them.

Found by the tests: the colour dodge and the colour burn turn on a channel being exactly
nought or one, and a value read back from an sRGB target is decoded and encoded again, so
one came back as 0.99999 and a burn over pure green painted black. The two ends are now
tested within far less than one 8-bit step (`EDGE` in the shader). The second pair of
colours in `blend_modes`, at the formulas' edges, is what showed it.

Mutation testing: 22 mutants, all killed, ten in the renderer by the pixels. Two survived
at first and the tests were sharpened:

- soft light over a dark destination: the source was too dim for the two formulas to
  differ by more than the tolerance, and is now nearly white;
- a blend beside a backdrop: a multiply against the placeholder texture is the source
  itself, so the test now asks for a difference, which inverts against it.

The rest cover a blend folding with nothing else, the blend kept when scaled, the
background group and its square fill, its clip, the border's separation, the mode
changing half way, the cross-over's sum, coverage, premultiplying, the sRGB space,
the burn's edge, the hue and gamut formulas, the mode's code, the replacing draw, and the
cut inside a group.
