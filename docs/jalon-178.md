# Jalon 178 — Table: frozen columns

## Analysis

A wide grid (many columns) overflows horizontally. You then want to **freeze** the first
columns (an identifier, a name) while the rest **scrolls horizontally** — the spreadsheet
"frozen columns" pattern. The table had neither horizontal scrolling nor a pinned column.

## Technical decisions

- **Composition: a frozen block + horizontal scrolling.** In frozen mode, the root becomes a
  `Flex` **row** of two blocks: on the left a `Flex` **column** of the frozen cells (header +
  rows, columns `0..n`), on the right a **horizontal** `Scroll` containing a `Flex` column of
  the remaining cells (columns `n..`). The **scrolling columns' header is inside the same
  `Scroll`**: it follows its columns as they scroll, while the frozen columns stay put. Reuses
  the existing `Scroll` — no new machinery.

- **Alignment by identical heights.** Both blocks are `Flex` columns with the same `gap` and
  rows of height `ROW_H`: row `r` of the frozen block lines up pixel for pixel with row `r` of
  the scrolling block. The blocks' widths (the sum of the columns + the gaps) complement each
  other to fit the total width.

- **A dedicated path, guaranteed regression-free.** `build_frozen()` only kicks in if the
  conditions are met (a total width + **all** columns fixed + `n` in `1..columns`, text, no
  virtualisation/checkboxes); otherwise it returns `None` and the table falls back on its normal
  layout. Existing tables (which do not call `frozen_columns`) are **unchanged**.

## Implementation

- `table.rs`: the `frozen` field + the `frozen_columns(n)` builder; `build_frozen()` (the frozen
  block + the horizontal `Scroll`) and `frozen_header_cell()`; a short-circuit at the top of
  `rebuild`.
- `goldens.rs`: `table_frozen_columns` (the "Name" column frozen, Q1/Q2 visible, Q3 out of
  frame).

## Verification

- **Unit**: `frozen_columns_split_into_pinned_and_scrolling_blocks` — a two-block root; a
  **horizontally** scrollable area (max_x > 0); a **frozen** cell clickable (selection); a
  **scrolling** header sortable.
- **Golden** `table_frozen_columns` **inspected**: "Name ▲" frozen, Q1/Q2 visible, Q3 cut off, a
  horizontal scrollbar, the rows aligned — no regression on the other 33 goldens.
- `cargo test --workspace` **green**.

## What's left

- **Frozen columns + virtualisation / checkboxes / widget rows**: mutually exclusive paths
  today; combining them (a large table both frozen **and** virtualised) would require nesting
  vertical (virtualised) and horizontal (frozen) scrolling — a dedicated milestone.
- A **separator shadow** between the frozen block and the scrolling area (a visual cue of the
  freeze), and **freezing on the right** (action columns): possible visual extensions.
