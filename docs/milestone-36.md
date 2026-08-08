# Milestone 36 — New widgets: Table, SegmentedControl, Toast

Three widgets, one of them built on the previous milestone's grid.

## Widgets

- **`Table::new(columns).header(&["Name","Score"]).row(&["Ada","5"])`** — a
  **text** data table, built on `Grid` (equal columns). A styled header (light
  background, muted text) + cells; it delegates its layout to the `Grid`. For
  rich cells → use `Grid` directly.
- **`SegmentedControl::new(sel, on_select).segment("Day").segment("Week")`** — a
  **controlled** segmented selector (connected buttons, the active one brought
  forward). Clicking the i-th emits `on_select(i)`.
- **`Toast::new("Saved").success()`** — a transient notification (card + accent
  bar according to the Info/Success/Error variant). The *widget* is passive; the
  *system* (timer, stacking) is the app's business.

## Demo — a showcase of the `update → Command` model

- A **`SegmentedControl`** replaces the list's three filter buttons.
- A **`Toast`** "Saved" appears bottom-centre when **Save** is clicked (a `Stack`
  layer), then **closes itself after 2 s** through a timed `Command`
  (`Command::perform(|| { sleep(2s); DismissToast })`) — demonstrating a delayed
  effect.
- A metrics **`Table`** (Widgets / Milestones) in the "About" tab.

## Tests

- `Table`: `columns × (1 header + N rows)` cells; the texts are painted.
- `SegmentedControl`: N segments; clicking the i-th → `on_select(i)`.
- `Toast`: card + accent bar (the variant's colour) + text.
- 72 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Table`: **text** cells; no sorting, row selection, or variable column widths
  (inherited from `Grid`'s limits).
- `Toast`: no built-in queue or stacking (the app handles that) and no dedicated
  entry/exit animation (the mount/unmount fade applies).
