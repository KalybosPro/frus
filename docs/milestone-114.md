# Milestone 114 — `Transform`: rotation (rotated composited layer)

## Analysis

The `Transform` widget's last transformation: **rotation**. Unlike translation
(J112) and scaling (J113), a rotation **does not preserve axis alignment** — a
rotated rect is no longer a rect. So for the first time an **affine transformation
in the render pipeline** was needed. This is an infrastructure milestone: it
equips the compositor with a reusable rotation pass.

## Technical decisions

- **Rotating a layer at compositing time, not each primitive.** The compositor
  already knew how to render a subtree **flat** into a texture and then compose it
  (group opacity, the save-layer mechanism). That path is reused: the layer is
  composited **rotated**. So a single pass rotates a whole subtree (rects, text,
  images, paths), **without touching the shaders of each** primitive type.

- **Counter-rotation in the fragment.** The texture holds the content flat, at its
  screen position. To paint the layer rotated by `+angle` about the pivot, the
  fragment samples at the position **counter-rotated by `-angle`**: the screen
  pixel `p` receives the content that, rotated by `+angle`, lands at `p`. Outside
  the texture after counter-rotation → transparent.

- **`Primitive::Layer` carries an `Option<LayerTransform>`** (angle + pivot in
  px). `None` = a layer simply composited (opacity). Followed by `scaled` /
  `translated` (the pivot scales and shifts, the angle is invariant).

- **Counter-rotated hit-testing.** A rotation cannot transform the click
  rectangles (they would stop being axis-aligned). So each of the subtree's click
  targets is marked with a counter-transformation `(angle, pivot)` instead; at
  test time the **point** is rotated by `-angle` before `contains`. Exact for a
  rotation; nested rotations keep the outermost one (a documented approximation).

- **API.** `Transform::rotate(radians)` (about the centre) and
  `Transform::rotate_from(radians, pivot)`. RTL correction: the world being
  flipped, the angle is inverted. `angle ≈ 0`: normal rendering (zero cost).

## Implementation

- `frus-core/scene.rs`: `LayerTransform { angle, pivot }` (+ `scaled` /
  `translated`); a `transform` field on `Primitive::Layer` (propagated through
  `scaled`/`translated`/`fade`/`layer`). Exported in `lib.rs`.
- `frus-gpu`: `LayerComposite`/`CompInstance` carry `(angle, pivot)`;
  `composite.wgsl` counter-rotates the sample (the fragment reads `viewport.size`
  → the viewport binding's visibility becomes `VERTEX_FRAGMENT`).
- `frus-widgets/widget.rs`: the `transform_rotate` trait method + forwards
  (`Box`/`Keyed`/`Responsive`/`animated`).
- `frus-widgets/transform.rs`: `rotate` / `rotate_from`.
- `frus-widgets/ui.rs`: the rotation block in `walk` (a rotated layer + marking
  the targets); `Hit` gains `xform` + `rotate_point`; `hit` / `long_press_at` test
  through `Hit::contains`.

## Tests

- `rotate_emits_a_rotated_layer`: `rotate(π/2)` produces a `Primitive::Layer` with
  `transform = Some(angle ≈ π/2, pivot = the child's centre)`.
- `rotate_hit_test_counter_rotates_the_point`: a 40×20 child rotated by +90° — a
  click at the **rotated** position (20, 25) hits the target, while the old
  position (35, 10) misses it.
- Suites green: frus-core 88, frus-gpu 16, frus-widgets 209; the whole workspace
  green. (The rotated rendering itself is not verified in CI — there is no GPU;
  the fragment's correctness is validated by construction, and the hit-testing by
  a unit test.)

## What's left

`Transform` now covers translation, scaling and rotation. Possible extensions:
non-uniform scaling (`scaleX`/`scaleY`), composing several transformations into a
single matrix, and an animated demo bringing the arsenal together (alignment +
`Tween` + `Transform`).
