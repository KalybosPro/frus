# Milestone 120 — Pixel tests for the transform pipeline

## Analysis

Since J114 (rotation) and then J117 (the unified affine matrix), the **rendering**
of transformations had only been verified *by construction*: `frus-widgets`'
tests prove the right `Affine` is emitted, but **nothing proved the GPU renders it
correctly**. This milestone closes that hole by actually rendering transformed
layers offscreen and **checking the pixels**.

## Technical decisions

- **It reuses the existing harness.** `frus-gpu::render_offscreen` (headless
  rendering + pixel readback) and `frus-test::render_scene` / `Snapshot::pixel`
  already exist, with a clean **skip** when there is no GPU adapter. No new
  infrastructure.

- **Pixel tests rather than PNG goldens.** We make **geometric** assertions
  ("after +90°, the bar is vertical, no longer horizontal") on pixels **at the
  heart** of the shapes, far from anti-aliased edges — robust from one GPU to
  another, self-documenting, with no binary image to commit.

- **We test the missing link: the shader.** A `Primitive::Layer` carrying a
  `LayerTransform` (an affine) is built directly and rendered. That exercises
  `composite.wgsl` — the sampling at `M⁻¹(p)` — end to end. Combined with the
  `frus-widgets` tests (that the right matrix is emitted), the whole chain is
  covered.

## Implementation

- `crates/frus-test/tests/transforms.rs`: a `transformed_layer(inner, color, m)`
  helper (a solid rectangle wrapped in a transformed layer) and four cases.

## Tests (run on the software rasteriser, **not ignored**)

- `rotation_turns_a_horizontal_bar_vertical`: +90° about the centre — the
  horizontal bar becomes vertical; the old location shows the background.
- `uniform_scale_enlarges_about_center`: ×2 — a point outside the original square
  but inside its image is painted.
- `non_uniform_scale_widens_x_only`: `scale(3, 1)` — widened in x, unchanged in y.
- `scale_then_rotate_composes`: ×2 **then** +90° in one matrix — the composed
  image (narrow and tall) is correct.
- The whole workspace green.

## What's left

- Extending this to **clipping** (ClipRRect/ClipOval) once it is built — the same
  pixel approach.
- PNG goldens for rich scenes (text + decoration) remain useful for overall visual
  regression; the pixel tests target the **geometry**.
