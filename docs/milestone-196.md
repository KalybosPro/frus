# Milestone 196 — Table: inline cell editing

## Analysis

`Table` could show text, widgets and checkboxes, freeze columns, virtualise… but **inline editing**
(clicking a cell to type into it, spreadsheet style) had never been demonstrated. The question: does
it need a new mechanism, or is it already composable?

## Technical decisions

- **No new mechanism — pure composition.** `Table::widget_row` already accepts a cell as an
  **arbitrary widget** (a `Fn() -> Box<dyn Widget>` factory). Inline editing therefore reduces to a
  choice of widget per cell, driven by application state:
  - a cell **at rest**: a clickable [`Container`] (`on_click`) showing the value — the click emits
    "edit cell (row, column)";
  - a cell **being edited**: a [`TextInput`] bound to the value (`on_input` → update, `on_submit` →
    commit).
  The application holds an `editing: Option<(row, col)>` and swaps the targeted cell's widget.
  Nothing to add to the framework: `Container::on_click` + `TextInput` + `widget_row` suffice.

- **This milestone is a proof of capability.** It pins the pattern down (and locks it with a
  golden) rather than adding code: `Table`'s flexibility (the data-table milestones) makes inline
  editing "free". The full interactive wiring (the `editing` state, commit/cancel) is a direct
  application of this pattern.

## Implementation

- `goldens.rs`: `table_editable` — a 3-column grid where every cell is a clickable `Container`
  **except** one, rendered by a `TextInput` (the cell being edited).

## Verification

- **Golden** `table_editable` **inspected**: the "Cryptographer" cell (row 2, Role column) is a
  bordered input field; all the others are clickable static text — inline editing composes with no
  framework code.

## What's left

- **Interactive wiring in the demo**: an "editable grid" route/section with
  `editing: Option<(row, col)>`, `on_input`/`on_submit` connected — a direct application of the
  pattern.
- **Keyboard navigation between cells** (Tab/Enter to move to the next cell) — app-level, or a
  future "grid" mode built into `Table`.
- **Per-cell validation** (an error border) — reusing `TextInput::error` + `Form`.
