# Jalon 48 — Drawer slide on a spring curve

The drawer slid **linearly** (the fixed duration of animated values). It now
follows a **spring curve**, for a soft arrival consistent with the app's screen
transitions.

## `spring_ease(t)` — the critically damped step response

A closed-form function that remaps linear progress `t ∈ [0,1]`:

```text
y(τ) = 1 − e^(−ω·τ)·(1 + ω·τ)     (ω = 8, renormalised so that f(1) = 1)
```

It is the **step response of a critically damped spring**: it starts at rest
(zero slope), rises decisively, decelerates gently, and **never overshoots**
(`f(0) = 0`, `f(1) = 1`, monotonic). Unlike `spring_step` (step-by-step
integration with a velocity, used for gestures and screens), this is a **closed**
form — no velocity state to keep, ideal for remapping a progress the runtime has
already interpolated.

No overshoot: essential for a panel docked to an edge (an overshoot would open a
gap at the window's edge).

## Application

`process_overlays` applies `spring_ease` to the progress of drawers only
(`Placement::Left` / `Right`) before deriving the slide offset and the scrim's
opacity. The other overlays (menus, tooltips, modals) keep their raw progress.

The runtime carries on driving the **linear** `0↔1` progress (no animation to
wire on the app side, milestone 46); the curve only comes in at **render** time.

## Tests

- `frus-widgets`: `spring_ease` — `f(0)=0`, `f(1)=1`, increasing, bounded at
  `≤ 1` (no overshoot), already well advanced at the halfway point, clamped
  outside its domain.
- The drawer's mid-animation test now derives its expectation from the curve
  (`spring_ease(0.5)·width`).

## Limits (v1)

- A single curve (critical damping); neither stiffness nor bounce is adjustable.
