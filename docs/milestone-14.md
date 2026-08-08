# Milestone 14 — Fade-out (retaining outgoing widgets)

Completes the lifecycle animations: a widget removed from the tree **fades out**
instead of vanishing in one step.

## Principle

A widget absent from the tree has no primitives left. So we make it disappear by
**snapshot + replay**:

1. **Tag**: every `Primitive` carries an `owner: u64` (= `WidgetId`), set by
   `build_ui` through `Scene::set_owner` before painting each widget.
2. **Detection**: `mounted (N-1) − present (N)` = the **outgoing** ids.
3. **Capture**: on the way out, the primitives whose `owner` ∈ outgoing are
   copied from the last scene → `Runtime.leaving[key] = (primitives, 1.0)`.
4. **Replay**: each frame, `build_ui` replays those primitives through
   `Scene::push_faded` with a falling opacity (`advance_leaving`, `1 → 0`), then
   forgets them at 0.

## What ships

- **`frus-core`**: `owner` on the primitives; `Scene::set_owner`;
  `Scene::push_faded(&Primitive, opacity)`; `Primitive::owner()`.
- **`frus-widgets`**: `WidgetId::as_u64`; `Runtime.leaving` +
  `Runtime::advance_leaving`; `build_ui` tags the `owner` and replays the
  outgoing widgets.
- **`frus-shell`**: captures the outgoing widgets from the last scene, advances
  the exit, and redraws for as long as an exit is in progress.
- **Demo**: a "− Remove" button; the removed item **fades out**.

## Loop (shell)

```
present  = collect_ids(&tree)
outgoing = mounted − present
for each exit: capture (last scene, owner ∈ outgoing) → runtime.leaving
mounts: new ids → opacity 0
advance (entry/hover/focus) | advance_leaving (exit)
ui = build_ui  (replays runtime.leaving, fading)
render ; if animating -> redraw
```

## Tests

- `Scene::push_faded`: reduced alpha, `owner` preserved.
- `Runtime::advance_leaving`: opacity falls towards 0, then the entry disappears.

## Simplifications (v1)

- The snapshot is **frozen** (the last frame's appearance and position): no
  layout and no internal animation during the exit — the standard behaviour of an
  exit animation. Outgoing widgets no longer receive events.
