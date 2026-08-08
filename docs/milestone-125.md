# Milestone 125 — **Per-corner** rounded clipping (`ClipRRect` + `BorderRadius`)

## Analysis

Shape clipping (J121) offered only a **uniform radius** (`RRect(f32)`). A radius
**per corner** (`ClipRRect(border_radius: BorderRadius::only(…))`) is the established
shape: a header with only its top corners rounded, an asymmetric bubble, a card whose
one side hugs an edge. This milestone takes rounded clipping to a **per-corner** radius.

## Technical decisions

- **`ClipShape::RRect` carries a [`BorderRadius`]** (4 radii `tl, tr, br, bl`) instead
  of an `f32`. `BorderRadius` is `Copy`, so `ClipShape` stays `Copy` (no ripple through
  the `Layer`s). A uniform radius is still `BorderRadius::uniform(r)`.

- **Quadrant selection in the shader.** `composite.wgsl` receives the 4 radii (a 5th
  instance attribute) and picks the one for the **fragment's corner** (`corner_radius`,
  identical to the rectangle painter `quad.wgsl`) before the rounded-rectangle SDF. Each
  radius is clamped to half the smaller dimension.

- **A backwards-compatible API.** `ClipRRect::new(f32)` (uniform) unchanged; a new
  `ClipRRect::rounded(BorderRadius)` for the per-corner form. The radii are `clamped()`
  (a negative radius makes no sense).

- **Transform tracking.** `ClipShape::scaled_xy` scales each radius (through
  `BorderRadius::scale`) — the clipping stays correct under a density change.

## Implementation

- `frus-core`: `ClipShape::RRect(BorderRadius)`; `scaled_xy` scales the 4 radii.
- `frus-gpu`: `LayerComposite` / `CompInstance` carry `radii: [tl, tr, br, bl]` (a 5th
  attribute); `composite.wgsl` selects the radius per quadrant (`corner_radius`).
- `frus-widgets`: `ClipRRect` stores a `BorderRadius`; `new` (uniform) +
  `rounded(BorderRadius)`.

## Tests

- `frus-test` (at the pixel level, on a real GPU):
  `rrect_clip_rounds_only_the_specified_corner` — only the top-left corner (radius 16)
  is erased, the other three stay **crisp**; the uniform cases and
  `RRect(0) = rectangle` still hold.
- `frus-widgets` / `frus-transforms`: the emitted clip shapes updated.
- The whole workspace green: frus-core 91, frus-gpu 16, frus-widgets 227, frus-test
  clip 4.

## What's left — `ClipPath` (an arbitrary path)

Clipping to an **arbitrary path** is a separate piece of work: it needs a **mask
pipeline** (render the path into a coverage texture, or a stencil buffer, sampled at
compositing) — not a simple extension of the fragment SDF, which only covers analytic
shapes (rect / rrect / ellipse). To be treated as its own brick (`ClipShape::Path` + a
mask texture per layer) rather than botched here.
