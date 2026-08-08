# Jalon 116 — `Transform`: composition (translate + scale + rotation)

## Analysis

The second half of completing `Transform`: **composing** several transformations
within one widget. Until now a `Transform` carried a single operation
(translation *or* scale *or* rotation). We want "grow **and** rotate" (a pop
effect that pivots), "offset **and** scale", and so on.

## Technical decisions

- **Chaining methods on the widget.** Alongside the single-operation
  constructors, `and_translate`, `and_scale` / `and_scale_xy` and `and_rotate`
  **accumulate** an operation without erasing the others.
  `Transform::scale(1.5).and_rotate(0.2)` carries both.

- **A fixed order of application: translation → scale → rotation.** The
  translation (already propagated through `child_offset`) is the **innermost**;
  the scale post-processes the flat primitives; the rotation wraps the whole thing
  in a rotated composited layer (the **outermost**). That is the natural order:
  position, resize, then pivot.

- **Merging the walk's two passes.** The separate "scale" and "rotation" blocks
  become **one**: the subtree is painted flat once, and then the scale (if
  present) *and then* the rotation (if present) are applied to the same range of
  primitives. So the rotation wraps primitives that are **already scaled**.

- **Consistent composed hit-testing.** The scale transforms the click rectangles;
  the rotation marks their counter-rotation. At test time the screen point is
  first counter-rotated (the outer rotation) and then tested against the scaled
  rect — the exact inverse of the paint order.

- **Pivots.** Each operation keeps its own pivot (an `Alignment`), resolved on the
  child's box. For the common case (pivots at the centre), the centre is invariant
  under centred scaling, so rotation and scale share the same centre; off-centre
  combinations are approximated (documented).

## Implementation

- `frus-widgets/transform.rs`: the `and_translate` / `and_scale` / `and_scale_xy`
  / `and_rotate` chaining methods.
- `frus-widgets/ui.rs`: the two blocks (scale, rotation) merged into one, which
  applies the scale and then the rotation to the same range.

## Tests

- `scale_and_rotate_compose`: `scale(2.0).and_rotate(π/2)` produces a **rotated
  layer** (angle ≈ π/2) *containing* an **enlarged** rectangle (~40×40) — both
  transformations composed, in the right order.
- The single-operation tests (translate / scale / scale_xy / rotate) stay green.
- Suites green: frus-core 88, frus-widgets 211; the whole workspace green.

## What's left

`Transform` now covers translation, scaling (uniform and per axis), rotation, and
their composition. Possible extensions: a true **single affine matrix** (merging
the three passes into one multiplication, hit-testing included) and an animated
demo bringing the arsenal together.
