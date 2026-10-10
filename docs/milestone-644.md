# Milestone 644 — Gradients, as the reference's

## Objective

The second of the three `BoxDecoration` milestones (after 641). frus's decoration had one
gradient: two colours, from the decoration's colour to an end colour, along a direction. The
reference has three (`painting/gradient.dart`), each with any number of colours:

| | reference | frus before |
|---|---|---|
| `LinearGradient` | `begin`, `end` (alignments), `colors`, `stops`, `tileMode`, `transform` | two colours along a direction |
| `RadialGradient` | `center`, `radius` (of the shorter side), `focal`, `focalRadius`, and the rest | none |
| `SweepGradient` | `center`, `startAngle`, `endAngle`, and the rest | none |
| `TileMode` | `clamp`, `repeated`, `mirror`, `decal` | clamp only |
| on a decoration | the gradient **replaces** the colour (`box_decoration.dart:419`) | the gradient started at the colour |
| in a transition | `Gradient.lerp`: two of a kind travel colour by colour; one alone fades by its opacity (`Gradient.scale`) | the far colour spread out of the fill |

## What was done

- **`frus_core::{LinearGradient, RadialGradient, SweepGradient, Gradient, TileMode}`** with
  the reference's fields and defaults:
  - linear from centre-left to centre-right;
  - radial centred, half the shorter side, no focal point;
  - sweep a whole turn from the right, clockwise;
  - clamp by default.

  `rotation` is the reference's `GradientRotation`, a turn about the box's centre.
  `Gradient::sample` is the colour at `t`, between the stops and held past them; stops that do
  not match the colours are spread evenly, as the reference's `_impliedStops`.
  `Gradient::resolve`, `scale` and `lerp`; `tile` for the four tile modes.
- **`BoxDecoration::gradient(impl Into<Gradient>)`** paints any of the three in place of the
  colour, in the box's corners or circle, with a uniform border.
  `Container::gradient(end, direction)` keeps working, as a two-colour linear gradient from
  the container's colour.
- **The scene** carries a gradient on a rectangle (`Primitive::Rect::shader`), described as
  the reference describes it: relative to the box, so it scales with it.
  `Scene::shaded_rect` paints one.
- **The renderer** bakes each gradient of a frame into one row of a ramp texture, 256 colours
  from its stops, and the rectangle shader:
  - computes where a pixel falls: along the line, between the two circles (the two-point
    conical form when there is a focal point), or round the centre;
  - tiles it;
  - reads the ramp.

  The ramp holds sRGB values and is filtered as such, so colours mix as written, as the
  reference's do; the shader turns the result linear. The two-colour gradient the
  framework's own widgets use is unchanged.

**Changed:** `LinearGradient` is the reference's, no longer `{ end, direction }`, and
`BoxDecoration::gradient` is an `Option<Gradient>`. A decoration's gradient replaces its colour
rather than starting from it, and a gradient on one side of a transition fades by its
opacity. A diagonal `Container::gradient` is now measured the reference's way, between two
opposite points of the box; the reference pictures did not move.

## Not done here

- A gradient's `transform` beyond a rotation.
- Gradients on paths: they keep their own two-colour linear and radial fades.
- `DecorationImage`, `backgroundBlendMode` and `blurStyle`: the next milestone.

## Tests

- `frus-core`:
  - colours sampled between their stops, held past them, and spread evenly when the stops
    do not match;
  - the four tile modes;
  - the reference's defaults;
  - lerp: two of a kind, one alone fading, two kinds swapping;
  - a decoration's gradient fading in by its opacity;
  - a faded gradient rectangle carrying the fade.
- `frus-test`: a new picture, `decoration_gradients`, checked by eye:
  - top row: three colours along a line with stops, a radial gradient, a radial one with a
    focal point, and a sweep;
  - bottom row: a line repeated, a line mirrored, a line turned an eighth, and a circle
    filled radially.

Mutation testing: eight mutants, all killed, the two in the renderer by the picture. They
cover stops that do not match the colours, the mix between two stops, a gradient fading out
alone, the mirror and decal tile modes, a faded gradient rectangle, the mirror code the shader
reads, and the radius measured on the shorter side.
