# Jalon 113 — `Transform`: paint scale (`scale`)

## Analysis

The `Transform` widget's second transformation: **scaling**. It grows or shrinks a
subtree **at paint time**, without touching layout — a button's "pop" on hover, a
thumbnail zoom, an icon breathing. Like translation (J112) it stays
**axis-aligned** (a scaled rect is still a rect), so **no matrix** is needed in
the GPU pipeline — unlike rotation, which is deferred to the next milestone.

## Technical decisions

- **Post-processing the range of primitives**, like the opacity layer. The subtree
  is painted normally, and then (through `Scene::split_off`) each emitted
  primitive is scaled **about a pivot** and reinserted in order.

- **Consistent hit-testing.** Scaling changes the geometry (unlike opacity): so
  the interaction rectangles emitted by the subtree are transformed **too** —
  click, long press, focus, dragging, scrolling, accessibility — with the same
  transformation. Rendering and hit-testing stay aligned.

- **The pivot is on the child's box.** The pivot is an `Alignment` (default: the
  centre) resolved on the **child's** box, not the `Transform`'s: the latter can
  be stretched by the parent (flex `stretch`), which would move the pivot away
  from the content actually being scaled.

- **Scene primitives: `translated` + `scaled_about`.** Added in `frus-core`:
  `Primitive::translated(dx, dy)` (mirroring `scaled`) and
  `Primitive::scaled_about(pivot, factor) = scaled(f).translated(pivot·(1−f))`,
  plus `Rect::scale_about` for the interaction rectangles. Scaling touches
  position, size, font, radii and strokes.

- **API.** `Transform::scale(factor)` (about the centre) and
  `Transform::scale_from(factor, pivot)`. A factor of `≈ 1.0`: normal rendering
  (zero cost).

## Implementation

- `frus-core/scene.rs`: `Primitive::translated`, `Primitive::scaled_about`.
- `frus-core/geometry.rs`: `Rect::scale_about`.
- `frus-widgets/widget.rs`: the `transform_scale() -> Option<(f32, Alignment)>`
  trait method + forwards (`Box`, `Keyed`, `Responsive`, `animated`).
- `frus-widgets/transform.rs`: `scale` / `scale_from` on the `Transform` widget.
- `frus-widgets/ui.rs`: the scaling block in `walk` (draining + transforming the
  primitives and the interaction surfaces).

## Tests

- `scale_grows_the_child_about_its_center`: `scale(2.0)` → a ~40×40 background,
  the same centre (10, 10).
- `scale_from_pins_the_pivot_corner`: `scale_from(2.0, TOP_LEFT)` → the top-left
  corner fixed at (0, 0), the background doubled.
- Suites green: frus-core 88, frus-widgets 207; the whole workspace green.

## What's left

- `Transform` **rotation**: an affine matrix passed to the shaders (vertex + SDF)
  and inverse-transform hit-testing — a render-infrastructure milestone.
- **Non-uniform** scaling (`scaleX` / `scaleY`) — a simple extension of the same
  post-processing if the need appears.
- A **scrollable** subtree inside a `Transform::scale`: the scrollbar's track is
  not transformed (a rare combination — not covered).
