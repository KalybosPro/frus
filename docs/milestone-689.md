# Milestone 689 — Paths painted like vector artwork

## Objective

The first step of #303 (SVG) and #304 (Lottie). Both describe artwork as paths painted with
gradients of any number of stops, in a space of their own, filled by a rule they name. A path
here could hold one colour or a two-colour fade, and was always filled even-odd.

## What was done

- **`FillRule`** on `Path` (`with_fill_rule`, `fill_rule`): `EvenOdd`, the default — what
  every path here has always been filled with, lyon's default — or `NonZero`, which most vector
  artwork uses and which a stroke expanded into an outline needs, its outline crossing itself.
  Every copy of a path (scaled, moved, mirrored, turned) keeps its rule.
- **`PathGradient::Ramp(Arc<GradientRamp>)`**: any number of `(offset, colour)` stops,
  `RampShape::Linear` or `RampShape::Radial` (between a focal circle and an outer one, the
  shape SVG and Lottie radial gradients take), a `TileMode`, and a `transform` from the
  gradient's space to the path's. `Scene::fill_path_ramp` fills with one. The fill colour
  multiplies it, so a faded primitive fades its gradient.
- **Drawn exactly under any transform**: each vertex carries its position mapped back into the
  gradient's space, and the fragment reckons the gradient there. The map is affine, so
  interpolating it across a triangle is exact; a turned, skewed or squashed gradient costs
  nothing more than a straight one. A scale or move of the primitive (the DPI scale, a
  translation) is composed into the transform.
- **The renderer**: the path pipeline reads the colours from a ramp texture of its own — one
  256-texel row per gradient in the frame, the box painter's scheme (milestone 644), with its
  helpers now shared — and repeats the box gradient's reckoning of `t`, tile modes and focal
  circle included.
- **`GradientRamp::t_at`, `sample` and `color_at`** do the same sums on the CPU: the tests hold
  the shader to them.

## Not done here

Strokes still take one colour, a cap and a width: for SVG and Lottie they are to be expanded
into outlines (joins, miter limit, dashes) and filled, which gives them gradients too. A sweep
gradient on a path, which neither format has.

## Verified

`frus-core`: the ramp along a line, the four tile modes, the stops mixed, the outward fade and
the focal one, a turned ramp, and a ramp following its primitive through a scale and a move;
the fill rule surviving every transform. On the GPU (`frus-test/tests/path_paint.rs`, the
laptop's GPU): a linear ramp, a radial one with a focal circle, and a turned, squashed and
repeated one, each read back at 64 pixels against `color_at` within 3 levels; nested squares a
hole under even-odd and filled under non-zero. Breaking the shader's repeat mode on purpose
failed the repeated case, so the test does read what is painted. The goldens on this GPU fail
the same 15 tests with and without the change (they are blessed under lavapipe); none is new.
