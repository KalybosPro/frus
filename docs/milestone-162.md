# Milestone 162 — Range slider: hover, track click & Home/End

## Analysis

`RangeSlider` (milestones 156/157/160) showed its tooltips **permanently**, did **not react
to a track click** (only the handles were interactive) and its keyboard support stopped at the
arrows. Three items from "What's left" to deal with.

## Technical decisions

- **A tooltip revealed on hover / focus.** The tooltip is now painted by the **handle** (not
  the slider), and only appears if the handle is **hovered** or **focused**
  (`status.hover_progress > 0` or `status.focused`), with a fade. The height stays reserved as
  soon as a `value_label` is set, but the display is contextual — as in Material. Each handle
  shows **its** value.

- **Click / drag on the track.** The parent `RangeSlider` becomes **draggable** again: since
  its handles are painted **on top**, `draggable_at` returns the handle when you aim at one,
  otherwise the **track** → `on_drag(fraction)` brings the **nearest handle** over (clamped,
  snapped). We get the track click back without breaking the handles' sticky dragging.

- **Home / End from the keyboard.** New shell routing: the **Home/End** keys are offered to
  the focused widget through `on_key` before editing (a text field ignores them here). A
  focused handle responds by running to its **bound** (0 / the neighbour, or the neighbour / 1),
  reusing `moved(±large)`.

## Implementation

- `slider.rs`: `RangeThumb` gains `label` + `value()` and paints the bubble **on
  hover/focus** (`paint_tip`); `on_key` also handles **Home/End**. `RangeSlider` passes `label`
  to the handles, no longer paints the bubbles, and becomes **draggable** (`on_drag` → the
  nearest handle, with `snap`).
- `app.rs` (shell): Home/End routed to the focused widget's `on_key` before the default
  action.
- `goldens.rs`: `range_slider_labels` **focuses** the low handle (revealing the bubble + the
  ring).

## Verification

- **Unit**: the track is **draggable** and `on_drag` targets the nearest handle (`0.25`→low,
  `0.9`→high); **Home/End** run the handle to its bound (low: 0 / 0.7; high End: 1). Sticky
  dragging, divisions, arrows, the height reserve: unchanged.
- **Golden** `range_slider_labels` **inspected**: the low handle **focused** with a ring and
  the "30%" bubble **revealed**, the high handle **without** a bubble; `range_slider`
  (unlabelled) **unchanged**.
- `cargo test --workspace` **green**.

## What's left

- **A tooltip during the drag**: hover/focus revelation does not yet cover a pure drag (no
  reliable "handle currently being dragged" signal — hover is lost); the actively dragged
  handle would have to be exposed from the shell.
- **PgUp/PgDn** (a big step) — would require dedicated `Key` variants.
