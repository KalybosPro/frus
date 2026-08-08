# Jalon 177 — Virtualised table: multiple selection

## Analysis

Virtualisation (milestones 173/176) excluded the **checkboxes**: a large virtualised grid could
not offer multiple selection. Yet that is the typical use of a table of thousands of rows (check
all, check a range). The checkbox column was needed **in virtualised mode**, keeping the "check
all" in the pinned header.

## Technical decisions

- **A box per row in the virtualised factory.** When `checkboxes` is on, the `List`'s row
  factory **prefixes** a `CheckCell` (as materialised rows do), aligned on the pinned header's
  "check all" box. `on_check` moves from `Box` to `Rc` so it can be captured in the `'static`
  factory.

- **"Check all" state based on the effective count.** `all_selected` / `some_selected` counted
  `self.rows` — **empty** when virtualised, so the header wrongly showed "unchecked". Fixed: a
  `row_count()` (the **virtualised** count when there is one) and an **O(selection)**
  `selected_count()` (unique valid indices) — the indeterminate state shows correctly even over
  millions of rows, without sweeping the whole range.

## Implementation

- `table.rs`: `on_check` as an `Rc`; a `CheckCell` prefixed in the virtualised factory; the
  `row_count` / `selected_count` helpers; `all_selected` / `some_selected` rewritten; the
  virtualised builders' docs updated (checkboxes now supported).
- `goldens.rs`: `table_virtual_checkboxes` (a checkbox column + an indeterminate "check all").

## Verification

- **Unit**: `virtual_table_supports_checkboxes` — "check all" in the pinned header emits
  `CheckAll`; a visible row's box emits `Check(i)`. `select_all_is_indeterminate…`
  (materialised) stays green (behaviour unchanged).
- **Golden** `table_virtual_checkboxes` **inspected**: a box per row, two rows checked, "check
  all" **indeterminate** (a dash) — no regression on the other 32 goldens.
- `cargo test --workspace` **green**.

## What's left

- **Variable row height when virtualised**: the `List` stays fixed-height (`ROW_H`) — the
  adaptive height (milestone 166) does not apply there; a `List` with per-index heights (prefix
  sums) would be a dedicated milestone.
- **Frozen columns / horizontal scrolling**: requires a horizontal viewport and a pinned column
  — a layout restructuring, a dedicated milestone.
