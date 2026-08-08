# Milestone 201 — Editable grid: keyboard navigation + rows

## Analysis

Milestone 197 wired up a **click-to-edit** grid: a single cell in a `TextInput` at a time. For a
real mini spreadsheet, the keyboard is missing (Tab from cell to cell, Enter to go down) along with
row management (add / remove). Rather than piling shortcuts onto the "one active cell" model, we
adopt the **spreadsheet** model: every cell is **always editable**.

## Technical decisions

- **An always-editable grid → Tab for free.** Each cell is a `keyed(("grid", r, c))` `TextInput`.
  The shell already navigates between **focusables** with Tab / Shift+Tab (the focus milestone), in
  tree order — so row by row, cell by cell. By making every cell focusable, **Tab becomes cell
  navigation** with not a line of shell code: we **compose** an existing brick. That also removes
  the `grid_edit` state (no more single "active" cell).

- **Enter = move down a row.** Each cell's `on_submit` emits `GridEnter(r, c)`; `reduce` returns
  `Command::focus(("grid", r+1, c))` if the next row exists, otherwise stays put. Typing now carries
  the coordinates: `on_input` emits `GridInput(r, c, value)`.

- **Adding / removing rows.** An "Add row" button (`GridAddRow`) pushes an empty row and **focuses
  its first cell**; each row carries, in its last column, a "✕" button (`GridDeleteRow(r)`). That
  button is a **non-focusable** `Container` (the trait's default): **Tab skips it**, so navigation
  stays cell to cell.

## Implementation

- `frus-demo/src/lib.rs`: `Msg::{GridInput(r,c,v), GridEnter(r,c), GridAddRow, GridDeleteRow(r)}`
  (replacing `GridEdit/GridInput/GridCommit`); the `grid_edit` field removed; `grid_screen`
  rewritten (always-editable cells, a delete column, an add button, the hint updated).

## Verification

- **Integration** (`grid_edit_navigate_and_resize`): typing updates the right cell; `GridEnter`
  moves down a row (a focus requested) and **stays** on the last one; `GridAddRow` adds an empty row
  (the right columns) and focuses it; `GridDeleteRow` removes the row, the following ones move up.
- **Manual**: in the grid, Tab / Shift+Tab walk the cells; Enter goes down; the buttons manage the
  rows.

## What's left

- **Enter on the last row → create a row** (instead of staying), arrow navigation, column sorting,
  per-cell validation (`TextInput::error` + `Form`).
