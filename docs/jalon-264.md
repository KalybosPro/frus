# Jalon 264 — Per-column vertical scrolling (Trello style), via an explicit height

## The goal

To complete the Trello pattern begun in milestones 258/260: the board scrolls **horizontally** (the row
of columns), and **each column** scrolls its cards **vertically**, independently. Milestone 263 found
that the natural approach (a `Scroll` at `flex(1)`) **collapses** for want of a defined ancestor height
— the cards disappear and are no longer reorderable. This milestone ships the feature through the
**documented stopgap**: an **explicit** height supplied by the application.

## The decision: an explicit height supplied by the app

Rather than waiting for a "fill-then-scroll" primitive in the layout engine, the **application**
supplies the card area's height — as a nested scrolling list often requires a defined height constraint.
It is **controlled** and overridable: without the call, the column keeps its original behaviour (the
content's height).

## Implementation

- **`frus-widgets/src/kanban.rs`**:
  - `Kanban::card_area_height(h)` (new): makes each column's cards **vertically scrollable** within a
    region of height `h`. `build_column` then composes the column in three zones — a **fixed title**
    above, the **cards + the drop zone** inside a `Scroll { axis: Vertical, height: h }`, a **fixed
    "+ Add card" button** below. Without the call (`card_area_height == None`), the cards stay direct
    children of the column (unchanged).
  - The `COL_PAD = 12` constant (extracted from the panel's padding): used to compute the inner width
    `COL_W − 2·COL_PAD` given to the `Scroll` and to its list.
- **`frus-widgets/src/flex.rs`**: `Flex::child_boxed(Box<dyn Widget>)` (new) — adds an already boxed
  child, to compose a dynamically built list (`Vec<Box<dyn Widget>>`).
- **`frus-demo/src/lib.rs`** (`board_screen`): computes
  `card_area = (height − BOARD_CHROME).max(160)` (reserving the navbar + the hint + the paddings + the
  title + the add button; a floor so it never collapses on a small screen) and passes it through
  `.card_area_height(card_area)`.

## Verification

- **Desktop**: compiles; widgets **395** (including the new guard), kanban unchanged, goldens **77**
  unchanged.
- **The guard (unit)**: `reorderables_inside_a_per_column_card_scroll_are_still_registered` — a column
  with a defined `card_area_height` puts its cards in a **vertical `Scroll` with a defined height** (the
  very case that collapsed in milestone 263); the visible cards stay **reorderable** (≥ 3: 2 cards + the
  drop zone). It complements `reorderables_inside_a_scroll_are_still_registered` (a board in a
  horizontal scroll, milestone 263).
- **On device**: to be confirmed by finger (a column's vertical scrolling + dragging a card within a
  scrolled column). The actual rendering and scrolling can only be verified on a GPU/device.

## A known limitation

The height is **explicit** (supplied by the app), not yet derived from an automatic "fill the available
height then scroll". Cards scrolled **outside** the visible region are not registered as reorderable
(expected: you do not drop onto an off-screen card without scrolling first). A reliable fill-then-scroll
primitive in the layout engine would make the stopgap unnecessary.

## What's left

- A "fill-then-scroll" primitive in the layout (which would remove the need for an explicit height).
- Vertical drag inertia (parity with the horizontal spring).
