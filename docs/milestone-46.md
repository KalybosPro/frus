# Milestone 46 — Drawer animation (slide + fade)

The `Drawer` from milestone 45 opened and closed instantly. It now **slides** in
from the left edge, scrim included, **with no wiring at all on the application
side**.

## Principle: animation driven by the runtime

The framework already interpolates a per-widget value towards the target declared
by `Widget::anim_target` (the mechanism behind switches and the like), through
`Runtime::advance_values`, called every frame by the shell. We hook into that:

- `Drawer::anim_target()` returns `Some(1.0)` when open / `Some(0.0)` when
  closed. The runtime drives the **progress** `0↔1` towards that target and
  requests another frame for as long as it moves — the app handles no spring at
  all.
- The `Drawer` **always** offers its overlay when a panel exists; it is the
  progress that decides whether it shows.

## Applying the progress

The `build_ui` walk reads the drawer's animated progress
(`Runtime::value_or(id, target)` — the target is the fallback on the first
render, as on mount) and attaches it to the overlay. `process_overlays` uses it
for `Left` placement:

- **Slide**: `pos.x = -(1 - progress) · width` (the panel comes in from the
  left);
- **Scrim fade**: opacity `0.5 · progress` (synchronised with the slide);
- progress ≤ 0 → no overlay emitted (no scrim, no panel, no dismiss zone).

The other overlays (menus, tooltips, modals) have no `anim_target`: their
progress is `1.0`, so their behaviour is unchanged.

## API additions

- `Runtime::value_or(id, default)`: the animated value, or `default` if the
  widget has never been animated (isolated render / mount).
- The internal overlay stack carries a `0..=1` progress.

No public `Drawer` signature changes: `Drawer::new(open)` is enough, and the
animation comes as a bonus.

## Tests

- `frus-widgets`: `anim_target` reflects the open/closed state; a closed drawer →
  no scrim; **mid-animation** (progress 0.5 injected) → the panel half in (right
  edge ≈ width/2).

## Limits (v1)

- No spring here: the interpolation is linear (a fixed duration shared with the
  other animated values) — sufficient and visually consistent.
- Still a single side (`Left`).
