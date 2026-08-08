# Milestone 119 — Animated showcase: `frus-transforms`

## Analysis

The first **tangible** demo of the transformation layer: a small runnable crate
(`frus-transforms`, on the `frus-hello` model) that **animates** the arsenal just
built — a composed [`Transform`] (rotation + scale), [`AspectRatio`] and
[`FractionallySizedBox`] — driven by a [`Tween`] over time. It is the first chance
to **see** rotation and scaling rendered by the GPU, beyond the headless tests.

## Technical decisions

- **A minimal Elm model.** The state = the elapsed time (`f32`). `update` advances
  the clock by a **fixed step** (`1/60 s`) → pure and testable. An
  `every(16 ms) → Frame` subscription keeps time (~60 fps) for as long as the
  window is open; each frame, the state changes and the `view` is rebuilt.

- **Animation driven by `Tween`, inside a pure `view`.** From the instant, a
  smoothed round-trip phase is derived (`Curve::ease_in_out`) and each value is
  interpolated by a `Tween`: scale `1.0 → 1.4`, fractional width `0.25 → 1.0`,
  plus a continuous rotation. No animated value is retained outside the time
  state.

- **A gallery of the `Transform` palette.** Two rows of tiles cover the whole
  range: `translate` (back and forth), `scale_xy` (squash/stretch, **non-uniform**
  scaling), `rotate + scale` (**composition**), `rotate @ corner` (an off-centre
  pivot, `rotate_from(TOP_LEFT)`) and `translate + rotate`. Then an
  `AspectRatio 16:9` box and a `FractionallySizedBox` bar that breathes.

- **Interactive.** A **clickable button placed inside a rotated `Transform`**
  increments a counter — *visible* proof that hit-testing crosses the
  transformation (the inverse matrix); a **slider** drives a scale live; and a
  **play/pause** button freezes the animation (and cuts the subscription). The
  whole thing scrolls (`Scroll`) so it stays usable in a small window.

- **Re-export.** `Alignment` (and
  `AlignmentGeometry`/`AlignmentDirectional`/`Affine`) are now re-exported by
  `frus-widgets` — applications need them for `Transform::rotate_from` /
  `Container::alignment`.

- **Project conventions.** Struct constructors only (`Text::new`,
  `Container::new`, `Flex::column`, `Transform::rotate`…), **no** free helpers;
  interface text in **English**.

## Implementation

- `crates/frus-transforms/`: `Cargo.toml` (rlib + cdylib, Android metadata),
  `src/lib.rs` (the `Showcase` app + desktop/Android entry points), `src/bin/`.
  Automatically included in the workspace (`members = ["crates/*"]`).

## Tests

- `frames_advance_the_clock`: `update` advances the time by a fixed step (pure).
- `ticks_continuously`: the subscription is never empty (a permanent animation).
- `renders_a_transformed_layer`: a **headless** render of one frame — the `view`
  does emit a **transformed** `Primitive::Layer` (the composed `Transform`),
  proving the showcase wires the stack end to end without a GPU.
- The suite green; the whole workspace green.

## Running it

- Desktop: `cargo run -p frus-transforms`.
- Android: an APK through `cargo-apk` (the same metadata as `frus-hello`).

## What's left

- Checking the rendering **on a real device** (desktop + Android) — the goal being
  to *see* the GPU rotation and scaling.
- The Web target (wasm + WebGPU).
