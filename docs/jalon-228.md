# Jalon 228 — Total on top of absolute stacked columns

## Analysis

A single-series `BarChart` writes the **value** above each bar (an immediate reading). **Absolute
stacked** bars had no numeric cue at all: you saw the composition but not the column's total. This
milestone restores **parity** — the total on top of each column.

Reserved for **absolute** mode: at 100% (milestone 224) the column is full by construction and
already labelled stratum by stratum (milestone 227), so a "100%" total would add nothing.

## Technical decisions

- **The total = the sum of the visible series.** At the end of the strata loop, `lower` already holds
  the sum of the category's **visible** series (hidden ones are skipped): we write it centred above
  the top stratum, at `top_y - VALUE_SIZE - 2`, exactly like a plain bar's value (the same
  `VALUE_SIZE`, the same `on_surface` colour, the same offset).

- **Nothing if the column is empty.** `lower > 0.0` avoids a stray "0" on a category with no visible
  data.

## Implementation

- `frus-widgets/src/chart.rs`: in `BarChart::paint`'s stacked branch, after the strata and only if
  `!normalized`, writing the column's total above.

## Verification

- **Widget** `stacked_absolute_bars_show_the_column_total`: two columns totalling 5 → the text `5`
  appears **twice** in absolute mode; at 100%, no raw total (shares in `%`).
- **Golden** `bar_chart_stacked` regenerated: the totals (5, 12, 11, 12, 7) on top of each column.
- Widgets 361; goldens 63.

## What's left

- Moving out of the charts domain (a new widget: an advanced `Calendar`/`DataTable`).
- A per-**stratum** value (inside the segment) in absolute stacked mode, like the `%` at 100%.
