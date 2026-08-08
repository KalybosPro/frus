# Jalon 161 — Reordering: keyboard & continuous sliding

## Analysis

Column reordering (milestones 153/155/159) was **mouse only** and its slide **jumped** from
one column to the next (the reflow followed the target column, not the cursor). Two gaps from
"What's left": **keyboard access** and a **smooth** slide.

## Technical decisions

### Continuous sliding (grouped per cell)

The old reflow classified each **primitive** by its position and shifted the band between
source and target as a block — hence a **jump** when the cursor changed column, and a risk of
**shearing** (a cell's background and text shifted differently during a transition).

The new `reflow_reorder_columns(prims, src, cursor_x, lifted_owner)`: we **group the
primitives by owner** (a cell = one `owner`: background + text + icon). Each cell slides **as
a block** (no more shearing) by a **continuous** amount driven by `cursor_x` — the slide
**follows the cursor** instead of jumping. The target is no longer needed for the preview
(only for the drop). Blocks wider than a column (page/row backgrounds) stay in place; cells
with no background (reduced to their text) take one notch's width as the transition scale.

### Keyboard reordering

The `on_key` routing (milestone 160) already offers the arrows to the focused widget. A
focused header consumes **Ctrl+Left/Right** (`Key::Left/Right { word: true }`) to move its
column by one notch (`on_reorder(from, to)`), **clamped** to the column count; at the edge it
**ignores** (the focus then navigates). **Bare** arrows stay ignored (focus navigation between
headers). Sorting by click/Enter is intact.

## Implementation

- `reorder.rs` (frus-widgets): `reflow_reorder_columns` grouped by `owner` + a continuous
  shift (the new `cursor_x` signature); tests updated (partial sliding).
- `table.rs`: `Cell.reorder` also carries the **column count**; `Cell::on_key` (Ctrl+Arrows →
  `on_reorder`, clamped).
- `app.rs` (shell): `paint_reorder_preview` reflows from `self.cursor.x` (no more target).
- `goldens.rs`: `table_reorder_preview` updated (the cursor past "Score", a full slide).

## Verification

- **Unit**: `reflow_reorder_columns` — a distant cursor → a **full** slide (col 1 → 0, col 2 →
  100); a cursor at a column's **centre** → a **half-way** slide (−50), the next one **still**;
  the leftward direction symmetric. `Cell::on_key` — Ctrl+Left/Right on the middle column →
  `Reorder(1,0)`/`Reorder(1,2)`; at the edges → `Ignored`; a bare arrow → `Ignored`.
- **Golden** `table_reorder_preview` **inspected**: "Role" lifted, "Score" (5/3) **slid** into
  "Role"'s place, the gap open, the "Role" ghost floating on the right.
- `cargo test --workspace` **green**.

## What's left

- **Temporal easing** (a spring) on top of cursor-following: would require a per-column
  animation state (an animated offset) in the runtime.
- **Announced keyboard reordering** (semantics/accessibility) and **PgUp/PgDn** to go to the
  edge.
