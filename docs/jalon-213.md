# Jalon 213 — LineChart: stacked areas

## Analysis

Milestone 209 overlays the series (comparison); to read a **total** and its **composition** (each
series' share of the sum), they must be **stacked** — cumulative areas, the classic form of a
composition-over-time chart.

## Technical decisions

- **`.stacked(bool)`.** Active with several series, each one becomes a **band** between its lower and
  upper cumulative totals; the bands add up from the bottom, the stroke following the upper edge. It
  implies the fill (a `STACK_ALPHA` opacity, stronger than a plain area so the strata are
  distinguishable).

- **The scale follows the total.** When stacked, `max` = `stacked_max` = the maximum of the **sum**
  of the series per category — so the axis (milestone 203) contains the whole stack.

- **A coherent tooltip.** On hover, the tooltip lists each series' **own** value (not the cumulative
  one); the accented markers are omitted when stacked (an individual height is meaningless on a
  cumulative stratum), the guide and the box remain.

- **No regression.** Without `.stacked`, the rendering path is milestones 200/206/209's (overlaid),
  unchanged.

## Implementation

- `frus-widgets/src/chart.rs`: the `stacked` field + `.stacked(bool)`; `stacked_max`; the stacked
  branch in `LineChart`'s paint (cumulative bands + the upper stroke); the tooltip marker guard; the
  `STACK_ALPHA` constant.
- `frus-test/tests/goldens.rs`: the `line_chart_stacked` golden.

## Verification

- **Unit** `stacked_areas_fill_a_band_per_series`: `stacked_max` = `max(2+3, 4+1) = 5`; two filled
  bands (solid paths with segments) when stacked, zero without.
- **Golden** `line_chart_stacked`: two cumulative bands, the scale at the total, a legend.

## What's left

- **Normalised** stacking (100%, relative shares), **smoothed** stacked areas (Bézier), and the same
  option for **BarChart** (stacked vs milestone 212's grouped bars).
