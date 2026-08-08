# Jalon 247 — Kanban: columns + cards, cross-column drag and drop

## Analysis

A **Kanban** board (titled columns of cards, moved by dragging) is a common application pattern. The
framework already has a **drag-to-reorder** mechanism (`reorder_index` / `on_reorder`): on drop, the
shell routes `source.on_reorder(target.reorder_index())`, **whatever** the two widgets are. This
milestone exploits it for card movement, with no new shell code.

## Technical decisions

- **A flat slot index.** Each slot `(col, pos)` carries a flat index `col * STRIDE + pos`
  ([`kanban_slot`]). That is a card's `reorder_index` — both as a **source** (grabbed) and as a
  **target** (dropped on). The grabbed card **decodes** the hovered target's index to emit
  `on_move(from_col, from_pos, to_col, to_pos)`.

- **A drop zone per column.** A zone at the bottom of each column carries the index `(col, card_count)`:
  the insertion target for the **end**, and the only target of an **empty** column.

- **Controlled.** The application holds the cards per column and applies the move (a removal + an
  insertion, correcting the index shift within a single column). The widget only renders and routes.

- **Customisable.** A column's panel background is **themed** (derived from `surface`/`on_surface`), not
  hardcoded.

## Implementation

- `frus-widgets/src/kanban.rs` (new): `Kanban::new(on_move).column(title, cards)`; the internal `Card`
  (a source+target, `reorder_index`/`on_reorder`), `DropZone` (the end target) and `Column` (a themed
  panel) widgets; the public [`kanban_slot`] helper. Tests: `slot_encoding_roundtrips`,
  `dropping_a_card_routes_a_cross_column_move` (a source card exposes its flat index and routes the
  right `Move` when dropped on another slot), `board_lays_out_one_widget_per_column`.
- `frus-widgets/src/lib.rs`: `mod kanban;` + exporting `Kanban`/`kanban_slot`.
- `frus-demo/src/lib.rs`: the `Board` route (+ the drawer/save-restore plumbing);
  `kanban: Option<Vec<Vec<String>>>` + the `kanban_cols` helper; `Msg::KanbanMove` + the reducer
  (removal/insertion, with the shift handled); `board_screen` over
  `Kanban::new(Msg::KanbanMove).column(...)`.

## Verification

- **Widgets**: slot encoding/decoding; dropping a card routes a cross-column `Move`.
- **Golden** `kanban`: three titled columns (To do/Doing/Done), cards as tiles, a drop zone at the
  bottom of each column — visually inspected.
- **Demo** `kanban_move_relocates_a_card`: moving a card removes it from the source and inserts it into
  the target; an in-column move reorders without duplicating (the index shift handled).
- Widgets 384; goldens 76; demo 36; the shell compiles.

## Notes

- The drag and drop reuses the shell's reorder preview (a ghost following the cursor), originally
  designed for `Table` columns — refining that preview for vertical cards remains a possible
  improvement.

## What's left

- A drop preview dedicated to cards (a vertical insertion indicator).
- **Rich** cards (widgets) rather than text; adding/removing a card.
