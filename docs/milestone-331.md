# Milestone 331 — Render a known colour and read the pixel back

Milestones 328, 329 and 330 were all the same bug wearing different clothes: a colour
specified correctly and painted as something else. A blend space, the disabled tokens,
every glyph in the framework. Each was found by accident — a photograph of a phone, an
arithmetic that stopped agreeing with a picture — and each of them was, in the end, one
sRGB↔linear conversion in the wrong place.

Three in a row is not bad luck; it is a missing test. This is the test.

## Why nothing caught them

The defect never reaches the layer anything was watching. The widget builds the right
colour, so the widget tests pass. The scene primitive carries the right colour, so a
`build_ui` assertion passes. The golden is *consistent* — it is regenerated from the same
broken pipeline, so it matches itself forever and looks fine to anyone who has not seen the
correct picture. Milestone 330's text was wrong from the day text was written, and 110
goldens agreed with it.

The only question that settles it is the one nothing was asking: **render a known colour
and read the pixel back.**

## Four surfaces, four conversions

They are not one path. Each does its own conversion, in its own place:

| surface | who converts |
|---|---|
| quads (`Primitive::Rect`) | `quad.wgsl`'s `srgb_to_linear` |
| text (`Primitive::Text`) | glyphon's shader |
| paths (`Primitive::Path`) | `path.wgsl`'s `srgb_to_linear` |
| images (`Primitive::Image`) | `image.wgsl`, on the *tint* |

That table is the bug. Milestone 330 was handing one of these a colour prepared for
another — the quads want linear because their shader converts, glyphon wants sRGB because
its shader converts, and the two look identical at the call site. So each surface gets its
own case, and the four are independent by construction.

The colour under test is `(0.4, 0.6, 0.25)`: mid-range, and different on every channel, so
a stray conversion moves it measurably and a channel swap shows up too. Near black or near
white both transfer functions flatten out and a slip hides in the rounding.

## It says what went wrong

A failure does not just report a mismatch; it names the conversion:

```
glyph: asked for [102, 153, 64], painted [34, 81, 13]
  — one sRGB→linear conversion too many, the shape milestone 330 had
(linearised would be [34, 81, 13], encoded twice [170, 203, 137])
```

Both candidate slips are computed and compared, because "the colour is wrong" sends you
looking at palettes — which is exactly the wrong place, and where this milestone's
predecessors each spent their first hour.

## Shown to bite, four times

A guard that passes vacuously is worse than none, and two things here could make it
vacuous: the no-GPU escape hatch, and the "strongest pixel" heuristic that finds a glyph's
core. So each case was made to fail on purpose:

- Reverting milestone 330's one-line fix fails **only** the glyph case, with the diagnosis
  above.
- Doubling `srgb_to_linear` in `quad.wgsl`, `path.wgsl` and `image.wgsl` fails those three
  and leaves the glyph case green.

Independent, and each pointed at its own surface.

## Left

- **Translucency is out of scope here** and stays that way: every colour in this file is
  opaque. An alpha still blends in linear light, which `blending.rs` pins and the roadmap
  tracks as its own question.
- **Gradients, shadows and the compositor's layer path** paint colours too and are not
  covered. The four here are the ones a widget reaches directly.
- **The clear colour** is checked only implicitly, as the thing every case measures
  distance from.
