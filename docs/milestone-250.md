# Milestone 250 — Reorderables registry (card dragging works)

## Analysis

The shell's drag-to-reorder located its targets through the registry of **clickable** widgets
(`ui.hit`). `Table` headers work because they are clickable (sorting), but **Kanban cards** and their
**drop zones** have **no** click action: they entered no registry at all, so the shell could neither
grab nor target them. The move (`on_move`, milestones 247–249) was logically correct but **did not
engage** with the mouse.

This milestone adds a **reorderables registry** separate from clicking.

## Technical decisions

- **A new `reorderables: Vec<(WidgetId, Rect)>` registry** in `Ui`, populated for every widget whose
  `reorder_index()` is `Some` — regardless of its clickability. Accessed through
  `Ui::reorderable_at(point)`.

- **Collected like `interactives`, not cached.** Rather than extending the paint cache
  (`BoundaryData`/`Snapshot`), we **disable** caching for a subtree containing a reorderable (through
  `plain_subtree_len`), exactly as for an `InteractiveViewer` or a `Scroll`: so the registry is rebuilt
  every frame, always up to date. **Transform** blocks (scale/rotation) also transform the
  reorderables' bounds (parity with `draggables`).

- **The shell uses the registry.** `reorderable_at` (the source on press), the **target** on drop, and
  the **insertion line** (the vertical preview, milestone 248) now read `ui.reorderable_at` instead of
  `ui.hit`.

## Implementation

- `frus-widgets/src/ui.rs`: the `reorderables` field (`Ui` + `Builder`) + init + assembly; the
  `if reorder_index().is_some()` collection in both walk loops; transforming the bounds in both
  `Transform` blocks; `plain_subtree_len` excludes reorderables from the cache; the `reorderable_at`
  accessor. The `kanban_cards_are_reorderable_without_being_clickable` test (a card is grabbable at the
  point **and** absent from the click registry).
- `frus-shell/src/app.rs`: `reorderable_at`, the drop target and `reorder_drop_line` go through
  `ui.reorderable_at`.

## Verification

- **Widgets**: the Kanban card is registered as reorderable without being clickable; `reorderable_at`
  finds it where `ui.hit` finds nothing.
- **No regression**: the registry emits no primitive (the same pixels) — the goldens unchanged;
  `Table` headers (reorderable **if** `on_reorder`) take the same path as before on the click side.
- Widgets 387; shell 26; goldens 77; demo 36.

## Notes

- Drag **engagement** (source/target/routing) is now correct and covered by unit tests (the registry +
  `reorderable_at` + the `on_move` logic). The **live** drag rendering (the ghost + the insertion line)
  remains runtime state not inspected on a GPU in this environment; a **rich** card's ghost only
  captures the tile (the content, painted by children, has a different owner) — a possible refinement.

## What's left

- A preview ghost including a rich card's **content**.
- A **between-cards** insertion cue (above/below depending on the hovered half).
