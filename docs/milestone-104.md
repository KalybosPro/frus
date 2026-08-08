# Milestone 104 — Composed `Animatable`s: `TweenSequence` + box tweens

## Analysis

J103 laid down the `Animatable` bridge (tween → live typed value). Two gaps
remained:

1. The **box properties** — padding (`Insets`) and radius (`BorderRadius`) — were
   not interpolable: no `impl Lerp`, so no `Tween<Insets>` and no
   `Tween<BorderRadius>`.
2. There was no way to chain **several stages** on one progress (a morph A → B →
   C, a grow-then-return bounce, segments with distinct rhythms).

## Technical decisions

- **`Lerp` for `Insets` and `BorderRadius`** — side by side (each padding /
  corner interpolated independently), like `Size`/`Point`. `Tween<Insets>` and
  `Tween<BorderRadius>` follow *for free* (the generic `Tween<T: Lerp>` covers
  them).

- **`TweenSequence<T>`** — a sequence of **weighted** segments. Each segment
  occupies a share of `[0,1]` proportional to its weight; `evaluate(t)` locates
  the active segment and evaluates it over its **local progress** `[0,1]`. The
  last segment catches the remainder (robust against rounding); zero weights → the
  last segment.

- **Arbitrary segments through `Box<dyn Animatable<Output = T>>`.** A segment is
  any `Animatable`: a `Tween`, a `Tween.curved(...)`, or even another
  `TweenSequence`. `Animatable` is *object-safe* (the `curved`/`animate` defaults
  are `where Self: Sized`, so they stay out of the vtable).

- **A `TweenSequence` is itself an `Animatable`**: it can be `.curved()` and
  `.animate(&controller)`d like any tween. Uniform composition.

- **Non-empty by construction.** `new(first, weight)` requires a first segment;
  `.then(next, weight)` adds more. So `evaluate` never has an "empty" case.

## Implementation

- `frus-core/animation/tween.rs`: `impl Lerp for Insets`, `impl Lerp for
  BorderRadius`; the `TweenSequence<T>` struct (`new`, `then`, `impl
  Animatable`). Importing `BorderRadius` / `Insets`.
- Re-exports: `animation/mod.rs` and `lib.rs` expose `TweenSequence`.

## Tests

- `insets_and_radius_tween_interpolate`: `Tween<Insets>` / `Tween<BorderRadius>`
  at the halfway point.
- `tween_sequence_relays_equal_weight_segments`: two equal segments hand over at
  `t = 0.5`, each traversed in full over its half (0/5/10/20/30).
- `tween_sequence_honors_weights`: weights of 3:1 → the seam at `t = 0.75`.
- `tween_sequence_drives_from_controller`: the sequence driven by a controller
  (colour black→white→black).
- The `frus-core` suite green (85).

## What's left

- A shell idiom / demo reading `sequence.animate(&ctrl).value()` in `view()`.
- `Tween<Alignment>` once alignment is introduced; a composite `decoration`.
