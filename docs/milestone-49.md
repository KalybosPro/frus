# Milestone 49 — Modal sheet (`BottomSheet`)

A **modal sheet** that slides up from the bottom of the window — the horizontal
counterpart of the drawer, for a batch of contextual actions or a short form
without leaving the current screen. It reuses **all** of the drawer's machinery:
overlay + scrim, runtime-driven progress, spring-curve arrival — **zero animation
wiring on the application side**.

## `Placement::Bottom`

The fifth overlay variant (after `Below`, `Center`, `Tooltip`, `Left`, `Right`).
Handled in `process_overlays` (`ui.rs`):

- **Axes**: width constrained to the window (`free_x = false` — the panel, at
  `Percent(1.0)` width, unfolds), natural height (`free_y = true`).
- **Position**: slides up from the bottom; the bottom edge stays stuck to the
  window — `y = window_height − progress · sheet_height`, `x = 0`.
- **Curve**: `spring_ease` applied to the progress (as for `Left`/`Right`) → a
  gentle deceleration with no overshoot.
- **Scrim**: a full-screen dark scrim modulated by the progress (a fade
  synchronised with the slide).

## `BottomSheet`

The same pattern as `Drawer` (modal mode only):

```rust
BottomSheet::new(app.sheet_open)
    .on_dismiss(Msg::CloseSheet)
    .sheet(actions_column) // the sheet's content
    .body(main_screen)     // the background, always visible
```

- An internal `SheetPanel`: full width, **natural height** (the content sets the
  height), a `surface` background, a top edge line + a centred rounded **grabber**
  handle, and 20 px of top padding to let the grabber breathe.
- `overlay()` returns the panel at `Placement::Bottom`; `anim_target()` follows
  `open` (`0↔1`) — it is the animated progress that decides the display and the
  slide (milestones 46/48).

## Demo

A "⋯" button in the header → opens a quick-actions sheet (Save / Clear completed
/ Close). Any action closes the sheet; so does the scrim. The sheet blocks the
back gesture (`can_go_back`), like the drawer and the modals.

## Tests

- `frus-widgets`: `anim_target` reflects being open, `Bottom` placement, no scrim
  when closed, scrim + full-width panel docked at the bottom when open, and the
  mid-animation slide derived from `spring_ease(0.5)·height`.
- `frus-demo`: the sheet toggles and closes on an action (`Save`,
  `AskClearDone`).

## Limits (v1)

- No drag-to-resize or drag-to-dismiss: the grabber is decorative. Opening and
  closing are programmatic only, plus the scrim.
- The natural height is not capped: very tall content can overflow the window (no
  automatic internal scrolling).
