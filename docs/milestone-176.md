# Milestone 176 — Virtualised table: widget rows

## Analysis

Virtualisation (milestone 173) was **text**: `virtual_rows` supplies strings per column. But
real large grids also show **widgets** per row (avatars, status chips, buttons) — which we want
to virtualise just as much (thousands of rich rows without building them all). The **widget**
variant was needed.

## Technical decisions

- **A unified row factory (`VirtualBuild`).** The virtualised mode now carries an
  `enum VirtualBuild { Text(Rc<Fn(usize) -> Vec<String>>), Widgets(Rc<Fn(usize) -> Vec<Box<dyn Widget>>>) }`.
  The `List`'s factory **matches** on it by index: text → a row of `Cell`s; widgets → a row of
  `WidgetCell`s. One virtualisation branch in `rebuild`, two public entry points.

- **`virtual_widget_rows`, mirroring.**
  `Table::virtual_widget_rows(count, viewport_height, build)` where
  `build(index) -> Vec<Box<dyn Widget>>` (one widget per column). Only the visible rows are
  built; selection by click (the cell catches the click under non-clickable content); the header
  pinned. The same exclusions as text mode (checkboxes / resizing / reordering).

## Implementation

- `table.rs`: `enum VirtualBuild`; `virtual_data` carries a `VirtualBuild`; `virtual_rows`
  wraps `Text`, the new `virtual_widget_rows` wraps `Widgets`; the `List`'s factory matches the
  type and builds a `Cell` or a `WidgetCell`.
- `goldens.rs`: `table_virtual_widgets` (500 rows of avatars + chips).

## Verification

- **Unit**: `virtual_widget_rows_builds_only_visible` — out of 3000 rows, **< 20** built; the
  pinned "Item" header + the "W0" widget painted; a widget row **clickable**.
- **Golden** `table_virtual_widgets` **inspected**: a pinned header, avatars + chips ("tag 1"…
  "tag 4"), a thin scrollbar (500 rows) — no regression on the other 31 goldens.
- `cargo test --workspace` **green**.

## What's left

- **Variable row height**: the `List` stays fixed-height (`ROW_H`); a taller widget would be
  cropped when virtualised (milestone 166's adaptive height does not apply there).
- **Virtualised checkboxes**: the multiple-selection column could be added to the row factory
  should a case call for it.
