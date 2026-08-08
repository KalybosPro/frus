# Milestone 23 — Scrolling with inertia (spring + bounce)

Scrolling goes from **discrete** (each wheel notch jumps) to **smooth**: the
wheel pushes a *target*, and the current offset springs towards it, with an
**elastic bounce** at the edges.

## The choice (being straight about it)

Touch "flick inertia" needs a **finger velocity**; here the input is a **wheel**
(discrete notches). So the right model is a **spring towards a target** plus
**rubber-banding** at the edges — not free friction. The advantages: it reuses
`spring_step` (the same language of motion as navigation and gestures), and it is
**deterministic and therefore testable** (friction constants, by contrast, would
be tuned by feel — impossible in software rendering without injected input).

## Mechanism

`Runtime` holds, per scrollable area:

- `scroll` — the **current** offset (what is rendered),
- `scroll_target` — the **aimed-at** offset,
- `scroll_velocity` — the spring's velocity.

Each frame (`Runtime::advance_scroll`, driven by the framework):

1. The **target** is pulled back towards `clamp(target, 0, max)` (elastic
   recall) — so an overshoot past the bounds comes back gently (the bounce).
2. The **current offset** springs towards the target through `spring_step`
   (K=200, C=28).
3. At rest (px thresholds), the animation state is cleaned up (the offset
   stays).

The inputs:

- **Wheel**: pushes the target, with an allowed overshoot of `SCROLL_OVER = 48 px`.
- **Scrollbar (dragging)**: stays **direct** (target synchronised, velocity
  killed) — a drag has to be precise, not elastic.

The `max` bounds come from the last `Ui::scrollable_maxes()` (stable from one
frame to the next → no latency: the advanced offset is rendered in the same
frame).

## Tests

- `scroll_springs_to_target_and_settles`: the offset reaches the target and then
  freezes (animation state cleaned up).
- `scroll_overshoot_rubber_bands_back_to_max`: a target beyond `max` comes back
  to exactly `max`.
- Total: **35 frus-widgets tests** + frus-demo + the doctest.

## Limits (v1)

- The feel is not finely tuned (no interactive test here); conservative
  constants.
- No true touch inertia (wheel input only) — the day a touch or trackpad input
  with a velocity arrives, we will seed `scroll_velocity` directly.
