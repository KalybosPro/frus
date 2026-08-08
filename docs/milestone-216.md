# Milestone 216 — BarChart: stacked bars

## Analysis

Milestone 212 gave us **grouped** bars (side-by-side comparison). Like the stacked LineChart
(milestone 213), BarChart must also know how to **stack** — a single bar per category, segmented by
series, to read a total and its composition.

## Technical decisions

- **`.stacked(bool)`.** Active with several series, each category has only **one** bar (the group's
  width), segmented from the bottom up: each series is a rectangle between its lower and upper
  cumulative totals, in its own colour (right-angled segments, radius 0, for a crisp stack).

- **The scale follows the total.** `max` = `stacked_max` = the maximum of the **sum** of the series
  per category — the axis (milestone 203) contains the whole stack. An exact mirror of
  `LineChart::stacked_max` (milestone 213).

- **Composes with the rest.** **Hidden** series (milestone 215) do not count towards the stack; the
  legend and the tooltip (each series' own value) work identically. Without `.stacked`, the grouped
  rendering (milestone 212) is unchanged, single series included.

## Implementation

- `frus-widgets/src/chart.rs`: the `stacked` field + `.stacked(bool)` on `BarChart`; `stacked_max`;
  the stacked branch in the paint (cumulative segments per category) vs the grouped branch.

## Verification

- **Unit** `stacked_bars_share_one_column_per_category`: `stacked_max = max(2+3, 4+1) = 5`; four
  segments (2 categories × 2 series) all at the group's **full width** (stacked in one column, vs
  grouped side by side). The `bar_chart_grouped` golden stays unchanged.
- **Golden** `bar_chart_stacked`: one segmented bar per category, the scale at the total.

## What's left

- **Normalised** stacking (100%), a rounded corner on the **top segment** only, and a tooltip
  tracking the **exact segment** under the pointer (not just the category).
