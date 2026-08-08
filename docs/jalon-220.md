# Jalon 220 — Charts demo: companion chart sharing the visibility

## Analysis

The selector (milestone 219) shows **one** chart at a time. To illustrate that several views can
react to the **same** state, the screen gains a **companion** chart: hiding a series through the main
chart's legend hides it in the companion too.

## Technical decisions

- **The complementary family.** The companion is always from the other family than the main chart:
  bars if the main one is lines, lines if the main one is bars. So you always see a *trend* reading
  and a *comparison* reading at once.

- **One constructor, an explicit `kind`.** `dashboard_chart` now takes `kind` as a parameter (instead
  of reading `app.chart_kind`). The main chart calls
  `dashboard_chart(app, app.chart_kind, …, legend = true)`; the companion
  `dashboard_chart(app, complement, …, legend = false)`. Zero duplication.

- **Shared state, no legend of its own.** Both charts read the **same** `chart_hidden`; the companion
  does not show its own legend (`legend = false`) — it simply **reflects** the visibility driven from
  the main one.

## Implementation

- `frus-demo/src/lib.rs`: `dashboard_chart` gains a `kind` parameter; `charts_screen` adds the
  companion (the complementary family, a reduced height, no legend) under the main chart.

## Verification

- `companion_chart_renders_across_families_with_hidden`: a (shared) hidden series; the screen renders
  with the main chart in lines (a bar companion) **and** in bars (a line companion). Demo 27/27.

## What's left

- Clicking a **point** on the main chart to pin its detail (milestone 221), and a **synchronised**
  clickable legend on the companion too.
