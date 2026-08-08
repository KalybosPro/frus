# Jalon 215 — Charts: clickable legend + hideable series

## Analysis

The legend (milestones 209/212) was decorative. A real dashboard lets you **click** an entry to
**hide/show** its series. Emitting a message on click forces the charts to become **generic over
`Msg`** — the brick that opens up every future interaction (clicking a bar/point).

## Technical decisions

- **Generic charts.** `BarChart` → `BarChart<Msg = ()>`, and likewise `LineChart`. The `()` default
  parameter keeps every existing construction inferable (goldens, doctests, tests) with no
  annotation. The only use site outside the crate: the goldens — verified unchanged.

- **The legend click routed as a sub-region.** `positional_click` (the `TextInput` suffix mechanism,
  milestones 198/205) reconstructs the legend's layout through `legend_hit` and returns
  `on_legend(index)`. The shell already routes it — no shell code.

- **Hiding on the data side.** `.hidden([indices])` removes the series from the plot (curves, areas,
  bars, stacked bands, the tooltip) while keeping their legend entry **muted** (so they can be
  brought back). The application toggles `hidden` in response to `on_legend`.

- **A stable scale.** `max_value` is still computed over **all** the series: hiding/showing does not
  make the axis jump.

## Implementation

- `frus-widgets/src/chart.rs`: the shared `legend_hit` helper; `BarChart` and `LineChart` made
  generic with the `hidden` / `on_legend` fields, the `.hidden` / `.on_legend` builders,
  `series_names`, skipping hidden series in every plotting path, muting in the legend, and
  `positional_click`.
- `frus-test/tests/goldens.rs`: the `line_chart_hidden` golden.

## Verification

- **Unit** `legend_click_emits_the_series_index` (a click on entry *i* → `on_legend(i)`, outside the
  band / with no `on_legend` → `None`); `hidden_series_is_not_drawn` (a hidden series = one line
  fewer). The 16 chart tests and the 58 goldens pass.
- **Golden** `line_chart_hidden`: "Costs" hidden (not plotted, muted in the legend).

## What's left

- **The demo**: a charts screen exercising the legend toggle (the domain has no demo screen yet).
  Clicking a **bar/point** (the same `positional_click`) for a detail view, and multiple series
  selection.
