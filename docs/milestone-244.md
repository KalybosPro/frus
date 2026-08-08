# Milestone 244 — DataTable: empty state ("No results")

## Analysis

With search (milestone 242) and bulk Delete (milestone 243), the table can end up with **no rows at
all** to show: too restrictive a filter, or every row deleted. An empty body topped by a "0 of 0"
footer is confusing; polished tables show an **empty state message**. This milestone adds one to
`DataTable`.

## Technical decisions

- **Detection at a single point.** The index pipeline (`sorted_order` → filter → sort → page) already
  yields the **total** of visible rows. When it is `0`, `rebuild` switches to the "empty" layout: the
  **header** (the columns stay readable) tops a **centred message**, and the **pagination footer is
  removed** (a pager over zero rows adds nothing).

- **An overridable message.** The default is **"No results"**; `empty_text(...)` allows a fitting text
  ("No people match your search") — in line with the customisability rule (a themed default, free
  override).

- **Automatic.** No extra API to wire app-side: as soon as the data/filter shows nothing, the empty
  state appears.

## Implementation

- `frus-widgets/src/datatable.rs`: the `empty_text` field (defaulting to "No results") + the builder;
  the `total == 0` branch in `rebuild` (the header + a centred message, with no footer); the
  `empty_filter_drops_rows_and_pager` test (a "zzz" filter → no clickable row **and** the pager removed
  — otherwise its single page button would emit a message).
- `frus-demo/src/lib.rs`: `data_screen` overrides `.empty_text("No people match your search")`; the test
  extended (a filter with no results still renders).

## Verification

- **Widgets** `empty_filter_drops_rows_and_pager`: a filter with no results → neither a row nor a pager,
  a non-empty tree (the header + the message).
- **Golden** `data_table_empty`: a "zzz" field, the header preserved, "No people match your search"
  centred, no footer — visually inspected.
- **Demo** `data_table_screen_…` extended: a filter with no results renders (the empty state).
- Widgets 380; goldens 74; demo 34; the shell compiles.

## What's left

- A confirmation before `Delete` (a dialog) in the demo.
- A new widget domain (an advanced `Tabs`, a `Tree` view, a `Kanban`).
