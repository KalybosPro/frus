# Milestone 103 — `Animatable`: the explicit → live typed value bridge

## Analysis

The typed interpolation base already existed — `Lerp` (number, colour, point,
size) and `Tween<T> { begin, end }.eval(t)` — but it was still **inert**: nothing
connected the `[0,1]` value an [`AnimationController`] produces frame by frame to
a typed tween. The view had to read `controller.value()` and then lerp by hand.

This milestone lays down the missing bridge, in the proven `Animatable` /
curve-tween / typed-animation shape: **one** `[0,1]` progress drives arbitrarily
many typed values, each with its own bounds and curve.

## Technical decisions

- **`Animatable` (a trait).** `type Output; fn evaluate(&self, t: f32) ->
  Output`. It is the abstraction tweens and curves share. `Tween<T: Lerp>`
  implements it (`evaluate = eval`).

- **`.curved(curve)` → `Curved<A>`.** It shapes `t` by the curve **before**
  evaluation: a linear progress drives a value with non-linear timing. Chainable
  on any `Animatable`.

- **`.animate(&controller)` → `Animation<'a, A>`.** It binds the animatable to a
  controller. `value()` samples the controller **at the present instant** — that
  is what the view reads at paint time, without otherwise knowing the controller.
  The controller's value is **normalised by its bounds**
  (`(v - lower) / (upper - lower)`), so a non-unit controller still drives a
  complete `[0,1]`. `Animation` also exposes `status()` / `is_animating()`
  (delegated).

- **Borrows, zero allocation.** `Animation` borrows `&animatable` and
  `&controller`: construction is free, it is disposable, and it is recreated on
  each `view()`. No rendering or platform dependency — it all stays in
  `frus-core`.

## Implementation

- `frus-core/animation/tween.rs`: the `Animatable` trait (+ the `curved` and
  `animate` defaults), `impl Animatable for Tween<T>`, the `Curved<A>` and
  `Animation<'a, A>` structs. Importing `Curve` / `AnimationController` /
  `Status`.
- Re-exports: `animation/mod.rs` and `lib.rs` expose `Animatable`, `Animation`
  and `Curved`.

## Tests

- `animate_follows_controller`: at rest at the bottom → `begin` (`Dismissed`);
  after a settled `forward` → `end` (`Completed`), on a `Tween<Size>`.
- `curved_reshapes_progression`: halfway through an `ease_in`, the value is
  **below** the linear midpoint; the bounds are reached (within the Bézier
  solver's tolerance).
- `non_unit_bounds_are_normalized`: a `[0,2]` controller at `1.0` → `t = 0.5` →
  the middle grey of a `Tween<Color>`.
- The `frus-core` suite green (81).

## What's left

- A shell idiom: instantiating an `AnimationController` per identity and reading
  `tween.animate(&ctrl).value()` in `view()` (a dedicated demo).
- Composed `Animatable`s (a `TweenSequence`, `Tween`s of `Insets` /
  `BorderRadius`).
