# Jalon 121 — Shape clipping: `ClipRRect` / `ClipOval`

## Analysis

Until now clipping was **rectangular**: each primitive carries a `clip: Rect`
tested in the fragment (`quad.wgsl`, `composite.wgsl`, …). The structuring brick
was missing: clipping a subtree to a **shape** (rounded corners, ellipse) — a
round avatar, a soft-cornered thumbnail whose image hugs the rounding exactly, a
circular badge.

## Technical decisions

- **Reuse the layer, no new pass.** A `Primitive::Layer` already isolates a
  subtree onto a texture and then composites it (the save-layer / opacity /
  transformation mechanism). A **clip shape** [`ClipShape`] (`Rect` /
  `RRect(radius)` / `Oval`) is added to it, inscribed in the layer's `clip`
  rectangle. Only the **compositing shader** changes — `quad`/`image`/`path`/`text`
  are untouched.

- **Coverage by signed distance, anti-aliased.** `composite.wgsl`'s fragment
  computes the shape's coverage: a crisp rectangle (kind 0, the original
  behaviour unchanged), a rounded rectangle through `sd_rounded_box` (kind 1), and
  an inscribed ellipse through a gradient-normalised approximate distance
  (kind 2). The curved shapes are softened over ~1 px (`smoothstep`), so the edges
  are clean at any scale.

- **Pass-through in layout.** `ClipRRect` / `ClipOval` take the size the parent
  gives them (as does their child); the shape is **inscribed** in that box, with
  the radius clamped to half the smaller dimension. No impact on the siblings.

- **The shape follows the transformations.** [`ClipShape::scaled_xy`] scales the
  radius (DPI, `Primitive::scaled`); translation leaves the radius and the ellipse
  invariant (only the `clip` rectangle moves). So the rounding stays correct under
  a density change.

- **Zero cost when unused.** `ClipShape::Rect` is the default: the existing layers
  (opacity, transformation) emit it and fall back exactly onto the old rectangular
  test.

## Implementation

- `frus-core`: the [`ClipShape`] enum (+ `Default`, `scaled_xy`); a `clip_shape`
  field on `Primitive::Layer` (propagated by `scaled_xy` / `translated` /
  `push_faded`); export.
- `frus-gpu`: `LayerComposite` / `CompInstance` carry `shape: [kind, radius, _, _]`
  (a 4th instance attribute); `composite.wgsl` computes the SDF coverage.
- `frus-widgets`: a new `clip` module — `ClipRRect<Msg>` (a radius) and
  `ClipOval<Msg>`; the `Widget::clip_shape()` method (default `None`) forwarded by
  `Box<dyn>`, `Keyed`, `Responsive` and the animated wrappers; the walk (`ui.rs`)
  wraps the subtree in a layer carrying the shape (like the opacity group).
  `ClipShape` re-exported.

## Tests

- `frus-widgets` (`clip`): the right `ClipShape` is emitted, the child is painted
  **inside** the layer, and the clip is **pass-through** (the sibling keeps its
  position).
- `frus-test` (`clip.rs`, **at the pixel level**, not ignored): `RRect(16)`
  **erases the corners** of the square while keeping the centre and the edge
  midpoints; `Oval` **keeps the inscribed disc** and erases the corners; `RRect(0)`
  **degenerates** into a full rectangle (a guard). Rendered on the software
  rasteriser — the shader really does apply the shape.
- The whole workspace green: frus-core 90, frus-gpu 16, frus-widgets 215,
  frus-test clip 3 + transforms 4.

## What's left

- **Per-corner** rounded clipping (a non-uniform `BorderRadius`) and `ClipPath`
  (an arbitrary path) — the same mechanism, a richer shape.
- `InteractiveViewer` (pan + pinch-zoom) on top of the transformation stack.
