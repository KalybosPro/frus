# Jalon 254 — Cross-cutting drag-and-drop review: Table + Kanban fixes

## Analysis

A cross-cutting review of the **drag-and-drop/reordering** domain (the shell + `reorder.rs` +
`kanban.rs` + `table.rs` + the `reorderables` registry), to lift the real defects — not just style. It
uncovered a **critical integration bug** that made milestones 248–253 **inoperative in the application**
on the Kanban side, plus several correctness/accessibility bugs now reachable.

## Fixes

### 1. `widget_rect` falls back on the reorderable registry (**critical**)
`Ui::widget_rect` only searched `focusables`. But **Kanban cards** (and headers that are reorderable but
**not sortable**) are not focusable: `widget_rect` returned `None`, so `paint_reorder_preview`
**returned immediately** (`let Some(src) = ui.widget_rect(id) else { return }`) — no ghost, no insertion
line, no vertical reflow — **and** the *insert-after* routing fell back to `false`. In other words, the
entire vertical preview (milestones 251–253) never ran with the mouse. `widget_rect` now has a
**fallback** on the `reorderables` registry (whose bounds already existed). That also fixes the case of
a `Table` header that is reorderable **without** sorting.

### 2. A screen-reader announcement that depends on the axis
The drop always announced "Column moved to position {to+1}". For a card, `to` is a **flat** index
(`col×STRIDE+pos`) → an absurd announcement ("position 1001") and the wrong noun. Now: horizontal → the
column position (1-based); vertical → "Card moved" (with no meaningless number).

### 3. Dropping a card **onto itself**
The `to != from` guard let through a drop on the grabbed card's **lower half** (where `to = from+1`) → a
**null** move message + a stray announcement (the reducer then cancelled them). A `self_drop` guard
(target == source) was added, neutralising a drop on oneself whichever half it lands in.

### 4. The horizontal spring restricted to the horizontal axis
The `reorder_x` spring (the columns' smoothed slide) was advanced for **every** reorder drag, including
vertical ones where it is **unused** — dead computation, and `reorder_animating` was watching the wrong
axis. It is now guarded to the **horizontal** axis (a new `dragged_reorder_axis` accessor).

### 5. A drop zone is no longer a drag **source**
`reorderable_at` (shell-side, on press) started a drag on any reorderable, including a `DropZone` — you
lifted an empty ghost that moved nothing. A new `reorder_draggable()` trait method (default `true`);
`DropZone` returns `false`. The **drop** still targets it (through `Ui::reorderable_at`), only the
**grab** ignores it.

### 6. A `STRIDE` overflow guard
`kanban_slot(col, pos)` carries a `debug_assert!(pos < STRIDE)`: beyond that, `pos` would overflow into
the column field (the flat index would silently target the next column).

## Verification

- **Widgets 392**: `widget_rect` falls back on the registry (a card found where `focusables` fails);
  cards grabbable **and** the drop zone target-only (`reorder_draggable`).
- **Shell 27**; **goldens 77 unchanged** (the preview only exists during a drag); **demo (lib) 36**;
  doctests 6.
- The announcement/self-drop/spring fixes live in the shell's `pointer_up`/tick (a stateful method), not
  isolable as pure tests without a full harness; their logic is simple and documented, and the
  `widget_rect` pivot (which unblocks them) is covered.

## Notes

- The **live** drag rendering remains uninspected on a GPU here; but critical bug #1 explains why
  milestones 251–253 could not be seen in the application — it is lifted.
- The review also raised **consolidation/style** points not addressed here (see What's left).

## What's left

- **Style**: the ghost's shadow colour (`Color::BLACK.fade`), the offset/blur and the insertion radius
  are literals in the DnD painting — to be moved onto the theme (the customisability rule).
- **Consolidation**: factoring out `ui.rs`'s two near-identical walk loops
  (`focusables/scrollables/draggables/reorderables/semantics`) and unifying `reflow_reorder_columns` /
  `reflow_reorder_cards` (the same idea on transposed axes).
- **Coverage**: a test of the **same-column** reflow (source/target overlap → a net zero shift), and a
  shell harness for the routing branches (`insert-after`, self-drop, the announcement).
- **Vertical** inertia/spring for the slide (parity with the horizontal).
