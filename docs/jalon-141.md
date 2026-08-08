# Jalon 141 — Up/Down arrows in the multi-line field

## Analysis

In a multi-line field, Up/Down should move the caret from one line to another. But the
shell treated those keys as **geometric focus navigation** — even from a text field — so
you left the field instead of moving up or down inside it.

## Technical decisions

- **The field decides, the shell arbitrates.** A
  `Widget::caret_vertical(width, cursor, down)` method yields the **new index** if the caret
  can change line **within** the field (same visual column), or `None` if it is already on
  the first (Up) or last (Down) line — or if this is not a multi-line field. The shell tries
  that move first; on `None`, it falls back to **focus navigation** (you leave the field).
  One piece of code therefore handles "move inside the field", "leave through the
  top/bottom" and "single-line field" (always `None` → navigation).

- **The same column, through the 2D layout.** The field shapes its wrapped layout, takes the
  current caret `(x, y)`, aims at the middle of the neighbouring line at the same `x`, and
  `hit_test` finds the index there. The visual column is thus preserved when moving up or
  down.

- **Selection with Shift.** As with Left/Right, `Shift`+Up/Down **extends** the selection
  (the anchor set at the start); without Shift, a plain move (the anchor cleared). The move
  **reveals the caret** (milestone 139), so the target line scrolls into view as needed.

- **Finding the focused field's geometry.** The vertical move needs the field's width (for
  wrapping). A `Ui::widget_rect(id)` accessor supplies it from the frame's focusables (not
  only the scrollable areas: a short, non-scrolling field navigates its lines too).

## Implementation

- `widget.rs` (+ the `Box`/`Keyed`/`Responsive` forwarders): the `caret_vertical` method.
- `textinput.rs`: the `caret_vertical` impl (wrapped layout → `hit_test` at the same column,
  `None` at the bounds / outside multi-line).
- `ui.rs`: the `Ui::widget_rect(id)` accessor.
- `app.rs`: the arrow block tries `caret_vertical` first (Up/Down), applies the move (+
  selection with Shift, + `reveal_caret`), otherwise navigates the focus.

## Verification

- **Unit**: from the 1st line, Down moves one line down, Up yields `None`; from the last,
  Down yields `None`; from the 2nd, Up moves back up; a single-line field always yields
  `None`.
- **No regression**: arrow focus navigation stays intact outside a multi-line field;
  `cargo test --workspace` green.

## What's left

- A **remembered goal column**: crossing shorter lines should retain the ideal column
  (editor behaviour) — here we restart from the current column at each jump.
- **Page Up/Down** and **Ctrl+Home/End** in the multi-line field.
