# Jalon 256 — Consolidation: transformed registries (ui.rs) + shared reorder factor

## Analysis

Milestone 254's review flagged two duplications in the drag-and-drop domain:
1. **`ui.rs`** — two near-identical blocks (the `walk` boundary and `emit_transformed_child`) captured
   the registries' bounds then, after compositing a transformed layer, counter-transformed the hit-test
   and mapped the focus/scroll/drag/**reorder**/accessibility rectangles. That is exactly where
   milestone 250 **forgot `reorderables`** in only one of the two blocks — the "you update one list, not
   the other" risk.
2. **`reorder.rs`** — both reflows share the "background vs cell" guard (`× 1.5`).

## Technical decisions

- **A single registry-transformation point.** A new
  `transform_interaction_registries(base, matrix)`: counter-transforms clicks/long presses (`M⁻¹`) and,
  if `matrix` is axis-aligned, maps the five registries' rectangles. Both sites call it after capturing
  `xform_base()`. An `XformBase` struct carries the base bounds — **distinct from `Snapshot`** because it
  **includes `reorderables`** (never cached, but definitely to be transformed). It is now impossible to
  forget a list in only one path: there is only one path.
- **The layer wrapping left in place.** The `split_off`/`Layer` differs by owner (`id` vs `owner`) and by
  the walk call (`walk_node` vs `walk`): kept inline at each site, only the **identical** (and fragile)
  part is factored out.
- **A shared `OVERSIZE_FACTOR`** for `reflow_reorder_columns`/`reflow_reorder_cards`. The **bodies** are
  **not** merged: the interaction model differs (columns = a **continuous** slide following the cursor;
  cards = a **binary** shift about the insertion line); fusing them would obscure both. Only the constant
  (the same idea, transposed axes) is shared, with a comment spelling out the relationship.

## Implementation

- `frus-widgets/src/ui.rs`: `struct XformBase`, `xform_base()`, `transform_interaction_registries()`;
  both transformed-composition blocks call the helper.
- `frus-widgets/src/reorder.rs`: `const OVERSIZE_FACTOR = 1.5` replaces the two `× 1.5`.

## Verification

- **A refactor with no behaviour change.** Widgets **392**, **goldens 77 unchanged** (the cached-boundary
  + transformed-layer paths — `RotatedBox`/`FittedBox`/`InteractiveViewer` — are covered and stay
  bit-for-bit identical), shell **27**, doctests **6**.

## Notes

- The helper centralises milestone 250's failure point: any new registry to transform under a layer is
  now added in **one** place.

## What's left

- Coverage of the **same-column** reflow (source/target overlap → a net zero shift).
- **Vertical** inertia/spring for the slide (parity with the horizontal).
- Unifying `Card`/`Toast`'s shadow onto `theme.scheme.shadow` (raised in milestone 255).
