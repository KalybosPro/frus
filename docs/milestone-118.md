# Milestone 118 — `Transform`: focus/a11y follow the scale (axis-aligned case)

## Analysis

The previous milestone (J117, the unified affine matrix) left a trade-off: under
a transformation, the **focus, scrolling, dragging and accessibility** rectangles
stayed **untransformed** (a general matrix cannot keep a rectangle axis-aligned).
That trade-off is lifted **in the common case** — when the matrix preserves axis
alignment (scale and/or translation, **without rotation**), a rectangle's image
*is* a rectangle, so it is computed exactly.

## Technical decisions

- **`Affine::is_axis_aligned`**: the linear part is diagonal (`b ≈ 0`, `c ≈ 0`) →
  no rotation and no shear.

- **`Affine::apply_rect`**: a rectangle's image under the matrix — exact when the
  matrix is axis-aligned (otherwise, the bounding box).

- **Conditional application in the walk.** After wrapping the transformed subtree
  in its layer, if the matrix `is_axis_aligned()`, `apply_rect` is applied to the
  emitted **focus / scrolling / dragging / accessibility** surfaces. When there is
  a rotation, they are left as they are (approximate bounds) — the **click** stays
  correct in every case.

## Implementation

- `frus-core/geometry.rs`: `Affine::is_axis_aligned`, `Affine::apply_rect`.
- `frus-widgets/ui.rs`: in the transformation block, re-capturing the
  focus/scroll/drag/semantics ranges and transforming them with `apply_rect` if
  the matrix is axis-aligned.

## Tests

- `axis_aligned_transform_scales_the_focus_rect`: a `Button` under `scale(2.0)` —
  a point outside the flat button but inside its enlarged image becomes
  focusable, and its focus rectangle is ~2× wider.
- Suites green: frus-core 90, frus-widgets 212; the whole workspace green.

## What's left

- Under **rotation**, focus and a11y stay at the unrotated bounds (the geometric
  limit of an axis-aligned rectangle) — the click, however, stays exact.
- An animated demo bringing the arsenal together (a `Tween` driving a composed
  `Transform`).
