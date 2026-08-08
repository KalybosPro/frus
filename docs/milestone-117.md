# Milestone 117 — `Transform`: unified affine matrix

## Analysis

Unifying `Transform`'s transformations. Until now, scaling went through
**per-primitive** post-processing (axis-aligned) and rotation through a
**composited layer**; composing them was approximate (off-centre pivots) and
non-uniform scaling fudged radii, text and paths. Scale **and** rotation are now
merged into **a single 2×3 affine matrix** (`Affine`), carried by the composited
layer — the exact transformation of a whole subtree, with no composition
approximation.

## Technical decisions

- **`Affine` in `frus-core`**: a 2×3 matrix (`[a, b, c, d, e, f]`) with
  `translation` / `scale` / `rotation`, a `then` composition, `about(pivot)`,
  `apply` and `inverse`. The unified type for paint transformations.

- **`LayerTransform` carries an `Affine`** (instead of an angle/pivot pair). The
  compositor computes the **inverse** and the fragment samples the texture at the
  counter-transformed position `M⁻¹(p)` — a single pass for any affine (per-axis
  scale, rotation, shear, composition).

- **A single pass in the walk.** The subtree is painted **flat**; the scale (about
  its pivot) and the rotation (about its own) are composed into `M`, and the whole
  is wrapped in a layer with `transform = M`. The per-primitive scale
  post-processing (J113/J115) disappears.

- **Hit-testing by inverse matrix.** The click targets carry `M⁻¹` (instead of a
  rotation pair); the test point is put through it. Exact for scale **and**
  rotation composed, in the right order.

- **What this lifts**: exact composition of off-centre pivots; correct non-uniform
  scaling (the flat content is stretched by the texture at compositing time, so no
  more fudging of radii, text and paths).

## Trade-offs

- **Scaling now goes through the GPU** (like rotation): so its rendering is **no
  longer verifiable without a GPU** (the primitives stay flat inside the layer).
  The tests therefore check the layer's **matrix** and the **hit-testing** (the
  inverse matrix), both without a GPU. The fragment's correctness is still
  validated by construction.
- **Focus / scrolling / dragging / accessibility** within a transformed subtree:
  those rectangles stay **untransformed** (a general matrix cannot keep them
  axis-aligned). **Clicks** stay exact (through the inverse matrix); the focus
  ring and the accessibility bounds appear at the untransformed position.

## Implementation

- `frus-core/geometry.rs`: the `Affine` type (+ export). `frus-core/scene.rs`:
  `LayerTransform` wraps an `Affine` (`rotation`, `scaled`/`translated` by
  conjugation).
- `frus-gpu`: `LayerComposite`/`CompInstance` carry the affine inverse (6 floats);
  `composite.wgsl` applies `M⁻¹` to `frag_px`.
- `frus-widgets/ui.rs`: the transformation block composes `M` and wraps the
  subtree in a layer with `M`; `Hit::xform` becomes an `Option<Affine>` (the
  inverse), and `contains` applies it.

## Tests

- `frus-core`: `affine_composes_scale_then_rotate_about_a_pivot`,
  `affine_inverse_round_trips` (an `M⁻¹∘M` round trip).
- `frus-widgets`: the scale/rotation tests check the layer's **matrix** (its
  linear part, its fixed point); `rotate_hit_test_counter_rotates_the_point`
  validates hit-testing by inverse matrix; `scale_and_rotate_compose` checks the
  merge into a single `rotation ∘ scale` matrix.
- Suites green: frus-core 90, frus-gpu 16, frus-widgets 211; the whole workspace
  green.

## What's left

- Transforming the **focus / a11y** rectangles too under an **axis-aligned**
  affine (pure scale/translation), to lift that trade-off in the common case.
- An animated demo bringing the arsenal together (a `Tween` driving a composed
  `Transform`).
