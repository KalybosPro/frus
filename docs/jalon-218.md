# Jalon 218 — Demo: "Charts" screen with a clickable legend

## Analysis

Milestones 209–217 made the charts rich, interactive widgets, but **no demo screen** exercised them
— the clickable legend (milestone 215) was covered only by a unit test. This milestone adds a screen
that **closes the loop**: a real `LineChart` in the app, whose legend drives the state.

## Technical decisions

- **A new routed screen.** `Route::Charts` joins the navigation (the drawer, the `5` shortcut, state
  save/restore). `charts_screen` renders a three-series `LineChart` (`Sales` / `Costs` / `Profit`),
  with an axis (`grid(4)`), a legend, and an animated hover halo (milestone 217).

- **The legend drives the state.** `.on_legend(Msg::ChartToggleSeries)` routes an entry click to
  `reduce`, which **toggles** the index in `chart_hidden`; `.hidden(app.chart_hidden.clone())`
  reflects the state in the plot. No shell code: everything goes through `positional_click`
  (milestone 215) and the app's Elm loop.

- **Nothing new widget-side.** The screen only assembles existing capabilities — the value is the
  end-to-end **integration** (a sub-region click → a message → state → a re-render).

## Implementation

- `frus-demo/src/lib.rs`: `Route::Charts` (+ the active index, save/restore, the drawer entry);
  `Msg::ChartToggleSeries(usize)`; the `chart_hidden: Vec<usize>` state; `reduce` (the toggle);
  `charts_screen`; the `LineChart` import.

## Verification

- `chart_legend_toggle_hides_and_shows_series`: the screen renders (`primitive_count > 0`); a click
  hides the series (`chart_hidden == [1]`), another hides a second (`[1, 2]`), a re-click brings the
  first back (`[2]`). The demo suite green; the workspace with no regression.

## What's left

- A **type selector** (lines / bars / stacked) on the screen, clicking a **point** to pin its detail,
  and a second chart (`BarChart`) sharing the visibility state.
