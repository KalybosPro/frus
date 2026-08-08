# Milestone 155 — Column reordering: sliding preview

## Analysis

Reordering (milestone 153) worked "blind": you grabbed a header, you released, the column
jumped. Without **visual feedback** during the drag, there was no way to aim at a position —
milestone 153 had left it in "What's left".

## Technical decisions

- **Painted by the shell, over the scene.** The drag is shell state (`Drag::Reorder`), not
  application state; so the preview must live **outside** the controlled tree. We reuse the
  inspector's pattern: clone the logical scene, **paint the preview**, then scale to physical.
  No preview data flows back into `view`, consistent with the architecture.

- **Three Material cues.** ① The **source column dimmed** (it is leaving its place);
  ② a **drop indicator** — a vertical `primary` bar at the target column's insertion edge
  (left if the target precedes the source, right otherwise); ③ a **lifted card** following
  the cursor: the header's box offset by `dx`, with a **drop shadow** and an **accented
  edge** (elevation, reorderable-list style).

- **Clipping neutralised.** The preview is drawn under `Rect::UNBOUNDED`: the card may
  overflow the source column without being cropped by the inherited clip.

- **Geometry through the existing hit-test.** The source's and target's bounds come from
  `Ui::widget_rect` (sortable headers are focusable → indexed). The target is resolved live by
  `reorderable_at(cursor)`. Zero new state.

- **Without text (accepted).** The card takes the box, not the label (the shell has no
  `label()` on widgets): a lifted rectangle + the indicator is enough to aim. Capturing the
  header's primitives for a ghost **including text** is noted in What's left (it runs into the
  clip stored per primitive).

## Implementation

- `app.rs` (shell): `draw_reorder_overlay(scene, theme, src, dx, drop)` — a **pure function**
  (the dim + an optional indicator + a shadow + the card); `paint_reorder_preview` computes
  the geometry (source, target, `dx`) and calls it; the rendering branch: if a
  `Drag::Reorder { moved: true }` is active, clone → paint → scale; `handle_drag` redraws on
  every move so the card follows the cursor.

## Verification

- **Unit** (`draw_reorder_overlay`, no GPU): with a target → **4** primitives (dim +
  indicator + shadow + card); with no target (the same column) → **3** (no indicator). The
  pure function isolates the shape from the event path.
- **Not a golden**: the preview is **interactive** (driven by the drag), not a static
  rendering; its shape is covered by the test above, and the routing (milestone 153) by the
  table's contract tests.
- `cargo test --workspace` **green**, with no warning.

## What's left

- A **ghost including text**: capture the header's primitives (`owner == id`), translate them
  and **un-clip** them (today each primitive carries its own clip).
- An **animated shift of the neighbours** (they part to open the drop slot).
