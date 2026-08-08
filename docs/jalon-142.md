# Jalon 142 — Remembered goal column + Page Up/Down

## Analysis

Two limits of vertical keyboard navigation (milestone 141) were left to lift:

1. **A "jumping" column.** Each Up/Down restarted from the *current* column. Crossing a
   shorter line, the caret stuck to that line's end, then went back down from that end —
   the original column was lost. Editors instead remember a **goal column** ("magic
   column"): you keep it as long as you only move up and down.
2. **No page jump.** PgUp/PgDn did nothing in a multi-line field.

## Technical decisions

- **The goal column carried by the shell.** A `TextInput` has no retained state; the shell
  keeps `goal_x: Option<f32>`. `Widget::caret_vertical` now takes that column as input
  (`goal_x`) and **yields the column to remember** for the next jump — `None` = restart from
  the current column. So the original column survives a short line: the wrapped layout
  `hit_test`s at the same `x`, which, clamped to a short line's end, puts the caret there,
  but the column returned stays the original one — and the next jump finds it again.

- **Explicit forgetting.** The goal column is **cleared** as soon as any other move
  happens: typing, deleting, Left/Right, Home/End (all go through `apply_key`, which resets
  `goal_x = None`) and a mouse click placing the caret. Only Up/Down/PgUp/PgDn preserve it.

- **Line vs page in a single method.**
  `caret_vertical(width, cursor, down, page, goal_x)` unifies both: `page=false` advances by
  **one line** (the caret height) and yields `None` at the bounds (the shell **navigates the
  focus**, as in milestone 141); `page=true` advances by **one page** (the field's visible
  height, ≥ 1 line) and **clamps to the field** — at the extremes the cursor settles at the
  start / end and yields `Some` (PgUp/PgDn never leave the field).

- **A shared factor shell-side.** The arrow block and the new PgUp/PgDn block call the same
  `App::move_caret_vertical(id, down, page)` helper: field geometry (`widget_rect`), the
  `caret_vertical` call, selection with Shift, remembering the column, `reveal_caret`. One
  path, two entries.

## Implementation

- `widget.rs` (+ the `Box`/`Keyed`/`Responsive` forwarders): the `caret_vertical` signature
  extended (`page`, `goal_x` → `Option<(usize, f32)>`).
- `textinput.rs`: a unified line/page impl with the goal column; clamping to the field in
  page mode.
- `app.rs`: the `goal_x` field; the `move_caret_vertical` helper; the PgUp/PgDn block;
  forgetting `goal_x` in `apply_key` and on the caret click.

## Verification

- **Unit**: the goal column crosses a short line (`"hello\nhi\nworld"`: col. 5 → "hi"
  clamped → lands far into "world"); PgUp/PgDn clamp to the field and yield `Some` at the
  extremes; milestone 141's cases (a simple line, bounds → `None`, single-line) stay green
  with the new signature.
- **No regression**: `cargo test --workspace` green, no golden moved.

## What's left

- **Ctrl+Home/End** (start / end of the field) and **Ctrl+Arrows** (word jump).
- The goal column is in **pixels**; moving to a "character" column later would be closer to
  fixed-pitch editors, but is pointless in a proportional font.
