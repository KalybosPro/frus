# Milestone 248 — Kanban: vertical drop preview

## Analysis

The cards' drag and drop (milestone 247) routes the move correctly, but the shell's **drag preview**
was designed for `Table` columns: the ghost only follows the **horizontal** axis (`dx`) and the
neighbours reflow as columns. For a card dragged **vertically**, the ghost did not move down and no
insertion cue appeared.

This milestone gives the shell an **axis hint** per reorderable widget and a **vertical** preview
branch.

## Technical decisions

- **`Widget::reorder_axis() -> ReorderAxis`** (default `Horizontal`). Additive: `Table` columns keep the
  existing horizontal preview unchanged; `Kanban`'s cards (and drop zones) return `Vertical`.

- **A vertical preview branch.** For a vertical axis, the shell: (1) makes the ghost follow in **2D**
  (`dx, dy`) instead of `dx` alone; (2) **does not apply** the horizontal column reflow; (3) lays down
  an **insertion line** (a `primary` band) at the top edge of the hovered slot (a card or a drop zone).

- **Pure, testable geometry.** The line's position is computed by
  `drop_insertion_line(target, thickness)` — a pure function, tested without a GPU (like
  `draw_ghost_card`).

## Implementation

- `frus-widgets/src/widget.rs`: the `ReorderAxis` enum + the `reorder_axis` method (default
  `Horizontal`) + forwarding in the `Box<dyn Widget>` impl; the export.
- `frus-widgets/src/kanban.rs`: `Card` and `DropZone` return `ReorderAxis::Vertical`; the
  `cards_declare_vertical_reorder_axis` test.
- `frus-shell/src/app.rs`: `paint_reorder_preview` branches on the axis (a 2D ghost + the insertion line
  when vertical, the horizontal behaviour unchanged); the `reorder_drop_line` helper + the pure
  `drop_insertion_line` function; the `insertion_line_sits_on_the_target_top_edge` test.

## Verification

- **Widgets**: the cards/zones declare the **vertical** axis.
- **Shell**: `drop_insertion_line` puts the band on the target's top edge, at its width.
- **No regression**: the horizontal branch (`Table` columns) is unchanged — the shell tests and the
  `Table` goldens unchanged.
- Widgets 385; shell 26; goldens 76; demo 36.

## Notes

- The drag preview is shell **runtime** state (it only appears during a drag): it cannot be captured by
  a golden (a **static** tree render). The **pure** parts (the axis, the line's geometry) are covered by
  tests; the **live** rendering is not inspected on a GPU in this environment.

## What's left

- **Rich** cards (widgets) + adding/removing a card in the Kanban.
- A finer **between-cards** insertion cue (above/below depending on the hovered half).
