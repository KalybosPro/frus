# Milestone 170 — Table: action widget in the header

## Analysis

A header could carry a label and, since milestone 168, a decorative icon — but the whole
header was only **one clickable area** (the sort). Real grids often put a **button** in the
header (a filter, a column menu) that must react **for itself**, without triggering the sort. An
**action widget** in the header was needed, clickable independently, **while keeping** sorting
and reordering on the rest of the cell.

## Technical decisions

- **The action is a *child* of the cell, not an overlay.** Rather than a floating layer (which
  would have required knowing the column edges, hence fixed widths), the action widget becomes a
  **child** of the header `Cell`, placed on the **right** (`justify: End`). The hit-test goes
  **deepest**: clicking the button returns **its** message; clicking elsewhere in the header
  returns the cell's (the sort). No edge geometry required — it works at any width (fixed **or**
  flexible).

- **A factory per column, called back at each rebuild.** `Table::header_action(col, make)`
  stores a `Fn() -> Box<dyn Widget>` factory (like the widget cells): since the table rebuilds
  itself at each setting, it produces a **fresh** widget. The sortable label, the icon and the
  sort indicator stay painted on the left; the action floats on the right.

## Implementation

- `table.rs`: the `Cell.action: Vec<Box<dyn Widget>>` field (0/1, exposed through `children`);
  `Cell::style` switches to `justify: End` when an action is present; the `Table.header_actions`
  field + the `header_action(col, make)` builder; wiring in `rebuild`.
- `goldens.rs`: `table_header_action` (a "Filter" button to the right of the "Status" header).

## Verification

- **Unit**: `header_action_widget_captures_its_click` — the header cell carries the action; a
  click on the button returns **its** message (`Filter`), a click elsewhere in the header
  **sorts** (`Sort(1)`).
- **Golden** `table_header_action` **inspected**: a "Filter" button to the right of the header,
  the ▲ sort indicator preserved on "Name" — no regression on the other goldens.
- `cargo test --workspace` **green**.

## What's left

- **Keyboard focus for the action**: the button is clickable with the mouse; reaching it on Tab
  assumes the supplied widget is `focusable` (which frus buttons are). Nothing to do table-side,
  worth keeping in mind for a keyboard-driven dropdown.
- A header ***entirely* replaced by a widget** (with no text label at all): a possible extension
  through a `widget_header`, should a concrete case call for it.
