# Milestone 230 — Value/share in each band (stacked areas)

## Analysis

Milestones 227/229 label every stratum of a stacked `BarChart` (the `%` share at 100%, the value in
absolute mode). **Stacked areas** (`LineChart`) only had those numbers on hover (milestone 226). This
milestone brings the same parity to the bands: at each category, the value (or the share) at the
band's centre, if it is thick enough there.

## Technical decisions

- **A band = strata per category.** A stacked area is continuous, but its "stratum" at category `i`
  is the thickness between `lower[i]` and `upper[i]`. We write a centred label there (horizontally on
  the point, vertically in the middle of the band) — the same logic and the same threshold
  (`STRATA_LABEL_SIZE + 4`) as the bars, the same `on_primary` colour.

- **Content per mode.** The share (`%` of `category_total(i)`) at 100%, the raw value in absolute
  mode. Consistent with the bars' stratum label.

- **Horizontal clamping.** Unlike the bars (inset by `BAR_FILL`), an area's vertices fall on the
  area's edges; the label's `x` is clamped to `[plot_left, plot_left + plot_w - lw]` so it does not
  overflow at the edge categories.

- **No click, no hover.** A static rendering, on top of the tooltip (milestone 226), which stays
  available.

## Implementation

- `frus-widgets/src/chart.rs`: in `LineChart::paint`'s stacked branch, after each band is plotted, a
  loop over the categories writing the value/share at the centre of the segments thick enough
  (horizontally clamped).

## Verification

- **Widget** `stacked_areas_label_each_band_with_value_or_percentage`: in absolute mode, the band
  values (`3`, `4`, `5`, `6`) are present; at 100%, `%` labels appear.
- **Goldens** `line_chart_stacked` (values) and `line_chart_normalized` (% shares) regenerated.
- Widgets 363, demo 32; goldens 63.

## What's left

- Moving out of the charts domain (a new widget: an advanced `Calendar`/`DataTable`).
- Tuning the labels' opacity/density when there are many categories (to avoid clutter).
