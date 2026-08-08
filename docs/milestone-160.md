# Milestone 160 — Range slider: value tooltip & keyboard

## Analysis

`RangeSlider` (milestones 156/157) was complete with the mouse but silent on two items from
"What's left": **no value displayed** (you dragged with no numeric cue) and **no keyboard
access** (the handles were neither focusable nor arrow-drivable).

## Technical decisions

- **An opt-in tooltip, always painted.** `value_label(fmt)`: a `primary` bubble above each
  handle shows `fmt(value)` (a percentage, a price…). We chose a **permanent** display (rather
  than hover-only): a handle's "active" state is unreliable **during** the drag (hover is
  lost, the interaction falls back to `Idle`). Painted by the `RangeSlider` itself (which
  knows `low`/`high` and the positions), not by the handles. **Without `value_label`, the
  rendering is unchanged** (height = `H`, the original golden intact).

- **A height reserve.** With a tooltip, the height becomes `TIP_H + TIP_GAP + H`; the track
  and the handles live in the **bottom** `H` band, the bubbles in the top zone — within the
  widget's bounds, so never cropped.

- **A generic keyboard route.** New shell routing: a **left/right** arrow is first **offered
  to the focused widget** through `on_key`; if it **consumes** it (`Handled`), the focus does
  not navigate. Reusable by any widget (not just the slider). The handles become **focusable**
  and respond to the arrows by moving their side by one **step** (one division if `divisions`,
  otherwise 5%), through the same clamped/snapped logic as the drag. The focus ring is
  accented on the focused handle.

## Implementation

- `slider.rs`: `RangeThumb` factors out `moved(delta)`/`snap`/`key_step` (shared by the drag +
  the keyboard), becomes **focusable** and handles `on_key`; it is drawn in the bottom `H`
  band. `RangeSlider` gains `label` + `value_label`, `content_h` (the reserve), paints the
  track/segment at the bottom and the **tooltips** at the top (`paint_tip`).
- `app.rs` (shell): the left/right arrows go through the focused widget's `on_key` before
  geometric focus navigation.
- `goldens.rs`: `range_slider_labels` (the "30%" / "70%" tooltips).

## Verification

- **Unit**: a right/left arrow moves the focused handle by one step (`divisions(10)`: 0.4 →
  0.5 → 0.3), the handles are **focusable**; `value_label` **increases** the height (the
  reserve). Sticky dragging, divisions, clamping: unchanged.
- **Golden** `range_slider_labels` **inspected**: "30%" / "70%" bubbles above the handles;
  `range_slider` (unlabelled) **unchanged pixel for pixel**.
- `cargo test --workspace` **green**.

## What's left

- **Revealing the tooltip on hover / focus** (permanent today): requires a reliable "active
  handle" signal during the drag.
- **Clicking the track** to bring the nearest handle over.
- **Home/End** (the bounds) and **PgUp/PgDn** (a big step) from the keyboard.
