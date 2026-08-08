# Milestone 249 — Kanban: rich cards + add/remove

## Analysis

The Kanban's cards (milestones 247/248) were only **text**. A real board carries **rich** cards (a
label, tags, action buttons) and lets you **add** / **remove** cards. This milestone adds per-card
widget content and the add affordance, keeping the controlled model.

## Technical decisions

- **Widget cards.** A card can host **widget content** (a factory called back at rebuild time, like
  `Table`'s widget cells) instead of a label. The card stays the tile (the background, the border, the
  drop source/target); its content paints on top. A new `column_widgets(title, factories)` builder,
  alongside `column(title, texts)`.

- **Adding per column.** `on_add(f)` places a **"+ Add card"** button at the bottom of each column;
  `on_add(col)` on click (the app adds the card).

- **Removal = rich content.** Removal is not a widget API: the **application** puts a **×** button in
  the card's content, emitting its own removal message. The widget imposes nothing — it renders the
  content it is given.

- **Controlled.** The app holds the cards (`data`/state) and applies the add/remove; the widget renders.

## Implementation

- `frus-widgets/src/kanban.rs`: `Card` gains a `content` field (rich widget or label); the internal
  `ColCards` enum (text or factories); the `column_widgets` and `on_add` builders; the bottom of a
  column carries the drop zone then, if requested, a "+ Add card" `Button`. The
  `rich_cards_host_content_and_add_button_is_present` test (the rich card keeps its reorder index and
  routes a Move; the clicks expose each card's × and the column's + Add).
- `frus-demo/src/lib.rs`: `Msg::{KanbanAdd, KanbanDelete}` + the reducers (adding at the bottom,
  removing the targeted card); the `rich_card(label, col, pos)` helper (a label + a danger ×);
  `board_screen` moves to `column_widgets(...).on_add(Msg::KanbanAdd)`.

## Verification

- **Widgets**: the rich card hosts its content, stays reorderable, and the add button is present.
- **Golden** `kanban_rich`: label + × cards and a "+ Add card" button per column — visually inspected.
- **Demo** `kanban_move_relocates_a_card` extended: `KanbanAdd` adds "New card" at the bottom;
  `KanbanDelete` removes the targeted card.
- Widgets 386; goldens 77; demo 36; the shell compiles.

## A known limitation (interactive dragging)

Card **movement** is correct and tested at the logic level (`on_move`, milestones 247/248), but its
**engagement with the mouse** does not fire in-app yet: the shell locates reorder targets through the
registry of **clickable** widgets (`ui.hit`), and cards and drop zones have no click action. `Table`
headers work because they are clickable (sorting). Making the cards genuinely draggable requires a
**reorderables registry** separate from clicking (in `ui`/the shell) — a dedicated milestone to come.
The **+ Add card** and **×** buttons (clickable) do work right now.

## What's left

- A **reorderables** registry (independent of clicking) to engage card/zone dragging.
- Card tags/colours, a card count per column.
