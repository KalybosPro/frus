# Jalon 167 — Accessibility: sort and selection announcements

## Analysis

Milestone 165 gave frus a **live region** (spoken announcements), wired to column reordering
alone. But two other table gestures change the state without telling a screen-reader user:
**sorting** a column and **checking** one (or all) row(s). Those had to be announced too — and,
more generally, a **reusable hook** was needed so a widget can declare what to announce when it
is activated.

## Technical decisions

- **A generic hook: `Widget::announce()`.** A new trait method (default `None`), returning the
  text to announce **when the widget is activated** (a mouse click **or** Enter/Space). It
  describes the **resulting** effect — not the current state — to match what the user wants to
  hear. Forwarded by `Box<dyn Widget>`, `Keyed`, `Responsive`, like the other methods.

- **The shell reads `announce()` at both activations.** On a **click's confirmation**
  (`pointer_up`, press == release) and on **keyboard activation** (Enter/Space), the shell reads
  the widget's `announce()` **before** `dispatch` (which rebuilds the tree) and pushes it
  through `set_announcement` (milestone 165's live mechanism).

- **The table predicts the effect.** A sortable header announces "Sorted by {label}
  {ascending|descending}" by **flipping** the current direction (ascending by default — the
  usual Material pattern). The checkbox announces the state **resulting** from its toggle: "All
  rows selected/deselected" (a header box) or "Row selected/deselected" (a row).

## Implementation

- `widget.rs`: `fn announce(&self) -> Option<String>` (default `None`) + forwarders (`Box`,
  `keyed.rs`, `responsive.rs`).
- `table.rs`: `Cell::announce` (the resulting sort), `CheckCell::announce` (the resulting
  selection).
- `app.rs`: reading `announce()` and `set_announcement` on the mouse-click and Enter/Space
  paths.

## Verification

- **Unit**: `sort_and_selection_are_announced` — an unsorted header → "Sorted by Name
  ascending"; already ascending → "descending"; a partial "check all" → "All rows selected"; a
  checked row → "Row deselected", an unchecked one → "Row selected".
- `cargo test --workspace` **green**.

## What's left

- **Row selection by click** (outside the checkbox): not announced — the data cell has neither
  the row identity nor the resulting state. To be wired if the app exposes those.
- **Sort prediction**: assumes an ascending/descending cycle; an app with an
  ascending/descending/none cycle would announce a direction one step ahead on the 3rd click.
