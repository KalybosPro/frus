# Jalon 219 — Charts demo: type selector

## Analysis

The Charts screen (milestone 218) showed only one chart type. A real dashboard lets you **choose**
the presentation. Every brick exists (lines, stacked areas, grouped bars, stacked bars) — all that is
left is exposing them behind a selector.

## Technical decisions

- **A `SegmentedControl` drives the type.** A `chart_kind: usize` state (0 lines, 1 stacked areas,
  2 grouped bars, 3 stacked bars); `Msg::SetChartKind` changes it. The selector reuses the existing
  widget (the same pattern as the task filter).

- **One constructor, four variants.** `dashboard_chart(app, height, legend)` builds the chart from
  `chart_kind`: a `LineChart` (with `.stacked` for the stacked area) or a `BarChart` (with `.stacked`
  for stacked bars). Every variant shares **the same data** (`CHART_CATS` / `CHART_SERIES` /
  `CHART_COLORS`), the axis, and the `chart_hidden` state — changing type does not lose the hidden
  series. The `legend` parameter prepares for a **companion** chart (milestone 220).

## Implementation

- `frus-demo/src/lib.rs`: the `CHART_CATS` / `CHART_SERIES` / `CHART_COLORS` data constants; the
  `chart_kind` state; `Msg::SetChartKind` + `reduce`; `dashboard_chart`; `charts_screen` gains the
  `SegmentedControl`.

## Verification

- `chart_kind_selector_switches_type_and_each_renders`: the default type (lines); each type (stacked
  areas, grouped bars, stacked bars) can be selected and **renders** (`primitive_count > 0`),
  exercising both branches of `dashboard_chart`. Demo 26/26.

## What's left

- Persisting `chart_kind` in `save_state`, and a second **companion** chart sharing the visibility
  (milestone 220).
