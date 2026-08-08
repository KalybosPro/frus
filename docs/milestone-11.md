# Milestone 11 — Animations (implicit transitions)

Introduces the frame clock, animation state retained by identity, and continuous
redrawing for as long as an animation is running.

## What ships

- **`Color::lerp`** (frus-core): colour interpolation.
- **`Runtime.anims: HashMap<WidgetId, f32>`**: one `0..1` progress per widget,
  with `advance_hover(dt) -> bool` (slides towards the target — 1 if hovered, 0
  otherwise — and reports whether an animation is still running).
- **`Status.hover_progress`**: read by `Container`, which interpolates
  `base.lerp(hover, ease(progress))` (**smoothstep** easing).
- **Animated loop** (shell): an `Instant` clock, a clamped `dt`, and a redraw
  requested again for as long as `advance_hover` returns true (paced by the
  Fifo/vsync present mode).

## Model

An **implicit** transition, the way CSS does it: when the hover state changes,
the widget's progress slides over time towards its target. That state is
retained **by identity** (`WidgetId`) in `Runtime` — the same infrastructure as
focus (J8), scrolling (J9) and the caret (J10).

```
RedrawRequested:
  dt = clamp(now - last_frame)
  animating = runtime.advance_hover(dt)     // updates anims[id]
  ui = build_ui(&tree, size, &runtime)      // anims -> Status.hover_progress
  render(ui)
  if animating -> request_redraw            // keep the loop going
```

## Demo

The button's hover colour now **fades** (~120 ms) instead of switching in one
step; the pressed state stays instant.

## Tests

- `Color::lerp`: exact midpoint; bounds.
- `Runtime::advance_hover`: progress rises towards 1 (hovered), settles (no more
  animation), falls back to 0, and then the entry is cleaned up.
- `Container`: progress 0 → base colour; 1 → hover colour.

## Limits (next milestones)

- A single progress per widget (hover). No mount/unmount animations yet, no
  custom curves, and no explicitly driven animations (a 0→1 controller).
- Focus and pressed stay instant.
