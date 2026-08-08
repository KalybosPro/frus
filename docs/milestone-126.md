# Milestone 126 — `InteractiveViewer`: inertia (fling) + pan bounds

## Analysis

The `InteractiveViewer` (J122) panned and zoomed, but two finishing touches were
missing: the pan could **push the content out of the frame** (nothing held it back), and
releasing mid-motion **stopped dead** (no momentum). This milestone adds **bounding**
and **inertia** (fling).

## Technical decisions

- **Pure, testable bounding.** `InteractiveView::clamped(viewport)` constrains the
  translation so the content (at the current scale) always **covers** the viewport: you
  cannot drag an edge of the content inside it. At scale 1 the pan is nil (the content
  fills exactly); below 1 (zoomed out) the smaller content is **centred**. Applied after
  every pan, every zoom, and every fling frame.

- **Decelerating inertia in the runtime.** An `interactive_velocity` map (px/s) carries
  the momentum of a released pan; `Runtime::advance_interactive(viewports, dt)` moves the
  translation, **bounds** it (touching an edge cancels that axis's velocity — no bounce),
  applies **exponential friction** and stops below a threshold. Driven frame by frame
  like scroll inertia, with the current frame's viewports.

- **Shell gestures.** `Drag::Pan` now tracks a **smoothed velocity** (an exponential
  average) and the **viewport** (bounding); releasing mid-motion starts the fling (past a
  threshold). A new press **or** a zoom **cuts** an ongoing fling (you take back
  control). Wheel zoom is bounded too.

## Implementation

- `frus-widgets`: `InteractiveView::clamped` (+ the `PAN_FRICTION` / `PAN_MIN_VELOCITY`
  constants); `Runtime::interactive_velocity` + `advance_interactive`;
  `Ui::interactive_bounds`.
- `frus-shell`: `Drag::Pan` enriched (smoothed velocity, `last_t`, `viewport`); bounding
  for the pan and the zoom; starting the fling on release; the per-frame
  `advance_interactive` call (aggregated with scroll inertia); press/zoom cuts the fling.

## Tests

- `clamped` (pure): the pan is **cancelled** at scale 1; bounded at the edge when zoomed
  in (the content always covers); **centred** when zoomed out.
- `advance_interactive` (runtime): a fling **decelerates, stops** and stays **bounded**
  (the velocity is cleaned up at rest).
- The whole workspace green: frus-widgets 231 (+4: 3 bounding + 1 fling), frus-core 91.

## What's left

- A configurable **boundary margin** (slack beyond the frame, elastic overscroll) — the
  bounding here is **strict** (margin 0, the usual default).
- **Two-finger pinch** (touch), once multi-touch is in place.
- Double-tap to zoom / reset (the customary shortcut).
