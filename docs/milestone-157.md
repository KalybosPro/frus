# Milestone 157 — Range slider: sticky handle & discrete step

## Analysis

`RangeSlider` (milestone 156) was a **leaf widget** using `on_drag(fraction)`: the fraction
moved the **nearest** handle. Two limits noted in "What's left":

- **Not sticky.** Once a handle was grabbed, crossing the other made the gesture "hand over"
  to it — Material keeps the grabbed handle **selected**.
- **Continuous travel** only (no discrete step).

The crux: `on_drag(fraction)` has **no memory** of the grabbed handle; a leaf widget cannot
know which one was taken on press.

## Technical decisions

- **Two separate draggable handles.** `RangeSlider` becomes **composite**: it paints the
  track + the active segment, and its children are **two `RangeThumb`s** placed along the
  track by shims (`Spacer`). Each handle is a **draggable widget** of its own: grabbing a
  handle drags **that** handle — the stickiness is **structural**, with no extra drag state.
  Crossing is ruled out by clamping (`low` clamped by `high` and vice versa).

- **Delta, not fraction.** Each handle uses `on_drag_delta(dx)` (milestone 151): `dx`
  converted to a fraction through the track width, **accumulated** onto the grabbed side's
  value. Unlike `on_drag(fraction)` — which would saturate on a handle's small box — the delta
  is insensitive to the widget's size. The API is unchanged: `on_change(low, high)` (the
  widget computes the absolute from its current state).

- **A discrete step.** `divisions(n)` snaps the dragged value to `k/n` (rounded), applied
  after clamping.

## Implementation

- `slider.rs`: `Spacer` (an inert shim); `Side` (low/high); `RangeThumb` (draggable,
  `on_drag_delta` → `on_change(low, high)` clamped + snapped); a composite `RangeSlider`
  (`divisions`, `rebuild` placing the handles, painting the track + segment, children = the
  handles).

## Verification

- **Unit**: each handle moves **its** side (+22 px = +0.1: low 0.2→0.3, high unchanged;
  −22 px: high 0.8→0.7); **sticky** — the low handle pushed all the way stops at the high one
  (0.8, 0.8) without pushing it; a null delta → no message; `divisions(10)` snaps +0.125 →
  **0.1**; `new(0.9, 0.1)` reorders into `(0.1, 0.9)`.
- **Golden** `range_slider` **unchanged** (pixel-identical rendering: the composite paints the
  same track + segment + two handles).
- `cargo test --workspace` **green**.

## What's left

- A **value tooltip** on hover / during the drag (a bubble above the handle) — requires an
  overlay conditioned on the hover state (today `overlay()` is structural, not driven by
  `Status`).
- **Clicking the track** to bring the nearest handle over (lost in the move to a composite:
  only the handles are interactive).
- **Growing the handle** on hover / focus, and **keyboard navigation** (arrows).
