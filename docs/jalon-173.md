# Jalon 173 — Table: virtualised rows

## Analysis

The table built **all** its rows at each rebuild (a `Flex` of rows). For a log or an export of
thousands of rows, that is a per-frame cost proportional to the **total** — unacceptable. The
framework already has a virtualisation primitive (`List`: a fixed item height, vertical
scrolling, items built on demand). It had to be **applied to the table**, keeping the header
pinned.

## Technical decisions

- **A pinned header + a `List` of rows.** In virtualised mode, the root becomes a `Flex` column
  `[header, List(count, ROW_H, build_row)]`: the header stays fixed, the `List` virtualises the
  data (a per-frame cost ∝ the **visible** rows). No new scrolling machinery — we reuse `List`.

- **A captured, `'static` row factory.** The `List`'s closure cannot borrow `self` (it outlives
  it). We **capture clones** of the parameters needed (columns, widths, total width, the selected
  set, `on_select` — passed as an `Rc`) plus the app's content factory
  (`index -> Vec<String>`). It builds a row of `Cell`s aligned on the same columns as the
  header.

- **An accepted v1 scope: text.** `virtual_rows(count, viewport_height, build)` supplies
  **strings** per row. **Selection** (click) works on the visible rows. Checkboxes / resizing /
  reordering / widget cells do not combine with virtualisation (off-screen rows have no retained
  state) — ignored in virtualised mode, documented.

## Implementation

- `table.rs`: `on_select` moves from `Box` to `Rc` (shared into the closure); the
  `virtual_data` field + the `virtual_rows` builder; the virtualised branch in `rebuild` (header
  + `List`); the free `col_dimension` helper shared by the direct and virtualised paths.
- `goldens.rs`: `table_virtualized` (1000 rows, a pinned header + the visible window).

## Verification

- **Unit**: `virtual_table_builds_only_visible_rows` — out of 5000 rows, **< 20** built (the
  visible window, not 5000); the pinned "Name" header + "R0" painted; the scroll bound =
  `5000 × ROW_H − viewport`; a visible row stays **clickable**.
- **Golden** `table_virtualized` **inspected**: a pinned header, rows 1..4 visible, a thin
  scrollbar (a lot of content) — no regression.
- `cargo test --workspace` **green**.

## What's left

- **Virtualised widget rows**: v1 is text; a `virtual_widget_rows` variant (an
  `index -> Vec<widget>` factory) would follow the same pattern.
- **Variable row height**: `List` v1 is fixed-height (`ROW_H`) — the adaptive height (milestone
  166) does not apply when virtualised.
- **Pinning the header during horizontal scrolling** and frozen columns: possible extensions.
