# Milestone 691 — Lottie animations

## Objective

#304: applications use Lottie animations (Bodymovin JSON, from After Effects or LottieFiles)
for loaders, onboarding, success marks and empty states. frus could not play them.

## What was done

- **`frus-lottie`**, a new crate. velato (default features off: no vello) parses a file and
  evaluates a frame; it draws through a trait this crate implements over frus's scene:
  - shapes as paths, filled non-zero;
  - strokes expanded into outlines by kurbo's stroker (widths, caps, joins, dashes), in the
    shape's own space as the reference renderer strokes;
  - flat colours, and linear and radial gradients as `GradientRamp`s (milestone 689) with the
    brush's transform;
  - layer opacity, clips and masks as clipped layers, and the blend modes of mattes through the
    layer blends of milestone 646 (`SrcIn`, `DstIn`, `Multiply`…);
  - a clip to an axis-aligned rectangle — the animation's frame, on every frame — as the
    scene's clip rectangle rather than an offscreen layer.
- **Older files, and the ones velato would crash on.** A survey over lottie-android's 134
  snapshot-test animations (Apache-2.0, one small file per feature) found velato refusing 61
  and panicking on 4 — and a panic, in an application built to abort on one, is the
  application gone. `normalize.rs` rewrites a file before velato sees it:
  - Bodymovin 4.x and early 5.x properties without their `"a"` flag, and keyframes that carry
    their end value (`"e"`) with a bare last keyframe, brought to the current form;
  - text layers left out (velato has none), so the rest of the animation plays;
  - solid layers (velato reads them and draws nothing) turned into the rectangle they are;
  - a missing or split rotation (3D layers) as the turn in the plane, a missing position as
    the origin, an unanimated split position as one position; an animated split position in
    a shape group refused with an error;
  - the Add and Hard Mix blends as Normal, and paths with no vertex removed.

  What is left out or changed is named by `Animation::unsupported`. After it: **134 of 134
  parse, none panics**; the only ones that draw nothing are text-only and one made of images.
- **`Lottie`** (`frus-widgets`, the new `lottie` feature, off by default): `memory`, `string`,
  `new`; sized like `Image`; `fit`, `alignment`, `opacity`, `semantic_label`. It **plays once
  from its first frame when it appears** and rests on its last; `repeat`, `reverse` (back and
  forth when repeating), `speed`, `animate(false)`; `progress(p)` for an application that
  drives it itself. The console says once what an animation uses that is not drawn.
- **`Widget::anim_initial`**: where a widget's animated value starts when the widget appears.
  `None` (every widget so far) keeps the old rule — at its target, no transition — and
  `Some(0.0)` sets off from there, which is what "play from the start" needed. Forwarded by the
  wrappers that forward `anim_target`.

## Not done yet (rest of #304)

- Text layers, image assets, sweep gradients, expressions.
- An animation is evaluated and its paths tessellated again every frame: a heavy one wants a
  cache.
- Measured on the test phone and in a browser.

## Verified

`frus-lottie`: seven tests on animations written by hand (size and length, a square following
its keyframes, fitting into a box, a stroke's outline, a gradient across a shape, a faded layer,
garbage refused) and six on the normalisation. `frus-widgets`: the widget's sizing, playing
once from the start, looping and swinging back and forth, speed, a driven moment, a broken
source; `anim_initial` in the runtime (from the start, without it, motion off). On the GPU
(`frus-test/tests/lottie.rs`), the square read back where the first and the last frames put it.
The survey (`frus-lottie/tests/survey.rs`, `LOTTIE_SAMPLES=<folder>`) over the 134 files, and
twelve of them rendered and looked at: masks, inverted masks, a matte, gradients, a repeater,
trimmed and stroked paths, precompositions, a layer blend, solid layers.
