# Milestone 339 — Three filters, and the coverage that was counted twice

The last subsystem milestone 336 counted, and the one with no precedent here: nothing in
this framework had ever transformed a subtree's *pixels*. Three of the five filters are
here — the three that filter **their own** content. The two that filter what is painted
*underneath* are a different and much more expensive question, and the next milestone.

## Where an effect over a subtree belongs

Nowhere new, as it turns out. A layer is already rendered on its own into a texture
before being composited — that is what makes group opacity correct — and that texture is
exactly a picture of a subtree. So the three effects are **fields of a layer**, not a
kind of primitive, and the paint walk reaches them through the same drain-and-wrap it
already does for an opacity group, a shape clip and a transform.

| | |
|---|---|
| `ColorFiltered` | a function of one pixel: greyscale, a tint, a contrast curve |
| `ImageFiltered` | a function of a pixel *and its neighbours*: blur, dilate, erode |
| `ShaderMask` | a two-stop fade blended over the result |

They are independent slots on one `LayerFilter`, in a fixed order — image, then colour,
then mask — because each is a filter of what the one before produced and the order is
the difference between a blurred tint and a tinted blur.

## Two filters, one layer

`ColorFiltered` around `ImageFiltered` is two widgets and would be two nested layers.
Compositing does not re-composite a layer found inside another one — a limitation this
renderer has always had — so the inner filter would simply not be applied.

So the walk **folds**: a filter whose subtree turned out to be a single neutral layer
(full opacity, plain rectangular clip, no transform) merges into it. The slots make that
well defined, and the merge **refuses** when both sides want the same one: greyscale of
an inverted picture is not the inversion of a greyscale one, so there is no single layer
that means both, and those two nest properly instead.

## The colour space, said once and meant

Blending and the colour matrix are defined on **sRGB-encoded** values, the space colours
are authored in. That is not a detail. The composite shader samples an sRGB texture, so
what it holds is *linear light*, and the two disagree by roughly a factor of two: a
greyscale matrix evaluated in linear light turns pure red into 0.48 rather than 0.21,
more than twice as bright. The fragment therefore un-premultiplies, converts to sRGB,
filters, and puts it all back — and skips the whole round trip when there is no filter,
where it is byte-for-byte what it always was.

A blur is the deliberate exception and stays in **linear** light, because a blur is an
average of light; averaging encoded values makes a bright edge over a dark one visibly
grey.

The test that holds this down samples the **rendered pixel**, not the scene: pure red
through a greyscale filter must read 54 out of 255. In linear light it would read 124.

## The coverage that was counted twice

Writing the mask found an older bug, and it had to be fixed here rather than noted for
later, because the mask is what makes it visible.

A layer texture is **premultiplied** — it was painted over a transparent target, which
is what the ordinary alpha blend leaves behind. The composite pass then handed that
premultiplied colour to the ordinary alpha blend again, which multiplies the colour by
the coverage a second time. On opaque content the coverage is 1 and nothing happens,
which is why 338 milestones went by; on an antialiased edge it darkened slightly; on a
mask fade it is unmissable, because a fade to nothing would run to **black** instead of
to the background.

The composite pipeline now blends premultiplied, and the fragment scales both halves by
the coverage. Half of white over black now reads 188 rather than 137, and there is a
test that says so in those words.

## Blend modes

Eighteen of them, the Porter–Duff set plus the separable blends, because three separate
things needed them: a `ColorFilter::Mode`, a `ShaderMask`, and — next milestone — a
backdrop. One `BlendMode` pays for itself three times.

## The blur, and why it is affordable

Separable: two passes of `2n+1` taps rather than one of `(2n+1)²`. A Gaussian is
separable exactly, and dilate and erode are too, because a maximum over a rectangle is
the maximum of the per-row maxima.

The tap count is **fixed** at twelve per side and the step scales with the radius, so a
40-pixel blur costs precisely what a 4-pixel one does. The Gaussian is truncated at
three standard deviations, past which a tap contributes under half a percent, which is
what makes tap `i` sit at `3i/12` sigmas whatever the radius — so the weights are
constants rather than something to recompute per frame.

The result is a fresh texture and becomes the layer's own, so the layer cache holds it
across frames. The image filter is part of the cache key: the same primitives blurred by
a different amount are a different picture, and an animated blur would otherwise hold
the first frame it drew for ever.

## `enabled` means no layer

All three widgets carry it, and it means *nothing at all* rather than a neutral effect.
A blur of zero is still a blur — a texture and two full-surface passes — so a caller
animating one down to nothing needs a way to say so. The reference documents the same
distinction, for the same reason.

## The mask is written in fractions, and stored in pixels

A caller says "from the top of the box to the bottom" because a caller knows the shape
of the effect and not the pixels the box will land on. A scene primitive cannot hold
that: a fraction of a box no longer knows which box. So `FractionalMask` is what the
widget takes and `MaskShader` is what the scene holds, and the walk resolves one into
the other at the moment the box is finally a place on screen — which is the same
absolute-geometry choice `PathGradient` already made, for the same reason.

## Left

- **`BackdropFilter` and `BackdropGroup`** — the two that filter what is painted
  *underneath*. The plan is to render everything below the layer into a texture with the
  machinery already here, put it through the same pre-pass, and let `BackdropGroup` share
  one such snapshot among the filters that ask for it. The next milestone.
- **`Baseline` / `IgnoreBaseline`** — taffy has baseline alignment; nothing reaches for
  it. After the backdrop, and that closes the catalogue.
- **Two filters of the same kind still nest**, and a nested layer is not composited. The
  fold covers the compositions that mean something; this one is the honest limit of it.
- **A blur is still cut off by whatever clips the widget.** The layer keeps the
  *ambient* clip rather than its own box, so a blur reaches past the pixels it came
  from — but a scrollable or a card above it still trims the bleed at its own edge.
