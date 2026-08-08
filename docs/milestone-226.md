# Milestone 226 — Percentages in the tooltip in 100% mode

## Analysis

100% stacking (milestone 224) shows **proportions**: the strata fill the whole height and the axis is
in percentages. But the hover tooltip still showed **raw values** — inconsistent with what the chart
foregrounds, and without giving the exact hovered share.

## Technical decisions

- **A shared `format_measure(value, percent_of)`.** A free function formats a tooltip measure: the
  raw value alone (`None`), or `value (share%)` when a 100% denominator is supplied. Both charts use
  it — the value stays visible, the share is added in parentheses.

- **The denominator = the hovered category's total.** In each tooltip, `percent_of` is
  `Some(category_total(hi))` in 100% mode (respecting `hidden`, clamped), `None` otherwise. So the
  displayed share is indeed relative to the column/category under the pointer, consistent with the
  strata.

- **No impact outside hover.** The change is confined to the tooltip path (`status.hover_cursor`): the
  goldens (rendered without hover) are unchanged.

## Implementation

- `frus-widgets/src/chart.rs`: the `format_measure` function; `BarChart`'s and `LineChart`'s tooltips
  compute `percent_of` from `normalized` and format each measure through `format_measure`.

## Verification

- **Widget** `normalized_bar_tooltip_shows_percentages`: hovering category A (two series at 2) → the
  tooltip contains `(50%)` at 100%, no `%` in absolute mode.
- **Widget** `normalized_line_tooltip_shows_percentages`: the same for the stacked areas.
- Widgets 359; the 63 goldens unchanged (the tooltip path is outside golden rendering).

## What's left

- Moving out of the charts domain (a new widget: an advanced `Calendar`/`DataTable`).
- A **share label** directly on the strata (inside the bar/band) in 100% mode.
