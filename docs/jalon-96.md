# Jalon 96 — Group opacity & `AnimatedOpacity`

## Analysis

The J92→95 arc had laid down all the pieces: composited GPU layers
([`Primitive::Layer`], J92), a layer cache (J94) and curved implicit animations
(J95). What was missing was the **widget** that brings them together:
`Opacity` / `AnimatedOpacity` — applying an opacity to **a whole subtree as one
piece** (and animating it). It was J92's last explicit "what's left" ("integrate
an Opacity widget into the walk, like `RepaintBoundary`").

Fading a subtree **primitive by primitive** (`push_faded`) would recreate the
double-blend that J92 fixes on overlaps. The right answer is a save-layer: render
the group onto a layer, then compose the whole layer at the wanted opacity.

## Technical decisions

- **Folded into `Container`** (the framework's idiom, like `repaint_boundary`)
  rather than a transparent wrapper: a wrapper "adopts" its child's node, and its
  animation scalar would collide with an animated child. `Container` gains
  `.opacity(o)` (fixed) and `.animated_opacity(o, duration, curve)` (animated) — a
  proven layout, its own node, its own scalar. No surprise layout node.

- **A new trait point**, `Widget::opacity_group() -> Option<f32>` (default
  `None`): it returns the group's **target** opacity. Combined with `anim_target`
  (animated opacity, J95), the fade runs on its own through `advance_values`.
  Forwarded by the wrappers (`Box`, `Keyed`, `Responsive`).

- **Draining in the paint walk** ([`crate::ui`]). On meeting a group: the subtree
  is painted normally into the scene, and then its range of primitives is
  **drained** ([`Scene::split_off`]) into a single `Primitive::Layer` at the group
  opacity. The effective opacity is the value **tweened** by the runtime (fixed →
  the target). **Fully opaque (≈1): no layer at all** (zero cost). Hit-testing is
  unaffected (the layer only touches the visuals).

## Accepted limits

- **Nested groups**: a `Layer` inside another's primitives is not recomposited (a
  limit inherited from J92).
- An **overlay** emitted *inside* the group (deferred out of the scene) is not
  faded.
- Opacity clamped to `[0,1]`; at ≈0 the subtree is still painted (and then made
  invisible by the layer) — simple and correct.

## Implementation

- `frus-core`: `Scene::split_off(start)` (moving a range of primitives).
- `frus-widgets`: the `opacity_group()` trait method + forwarders; `Container`
  (the `opacity`/`opacity_anim` fields, the `.opacity`/`.animated_opacity`
  builders, `opacity_group()` + `anim_target`/`anim_duration`/`anim_curve` when
  animated); the `ui::walk` wraps the subtree in a layer.

## Tests

- `frus-widgets`: `opacity_group_wraps_subtree_in_a_layer` (the scene contains a
  `Layer` at 0.5 wrapping the content); `full_opacity_emits_no_layer` (full
  opacity → no layer); `animated_opacity_declares_anim_target` (target/duration/
  curve exposed; a fixed `opacity` → no animated value).
- `frus-test`: `group_opacity_fades_the_box` — **an end-to-end pixel proof**
  (widget → walk → layer → GPU): `opacity(0.5)` visibly dims the red compared with
  `opacity(1.0)`.
- Goldens and the existing suites unchanged: the path only fires if
  `opacity_group()` is `Some`.

## What's left

- **Named** `Opacity`/`AnimatedOpacity` widgets (sugar over `Container`), and
  `Animated*`s interpolating other properties (colour/size/padding) through
  [`Tween`] — the next step of implicit animations.
- Recompositing nested groups; fading overlays.
