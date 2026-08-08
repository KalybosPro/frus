# Jalon 227 — Share label (%) in each band (100% bars)

## Analysis

In 100% stacking (milestone 224), the strata show the proportions, but the exact value was only
readable on hover (milestone 226). For a 100% bar chart, the convention is to write the **share (%)
directly inside each stratum**: an immediate reading, with no interaction.

Reserved for **bars**: each stratum is a discrete rectangle that can host a centred label. Stacked
areas (lines) have no such clean per-category division — one label per band/category would clutter
them; they keep the share on hover (milestone 226).

## Technical decisions

- **The share at the stratum's centre.** In `BarChart::paint`'s stacked branch, in 100% mode, each
  visible segment gets `{share}%` (rounded) centred horizontally (on `cx`) and vertically (the middle
  of `[y_top, y_bottom]`).

- **A height threshold.** The label is only drawn if the stratum is at least `STRATA_LABEL_SIZE + 4`
  px tall — too thin a share stays untexted (it would be unreadable otherwise), the value staying
  available on hover.

- **Text readable over a saturated background.** The colour is `theme.on_primary` (the theme's "text
  on a coloured surface" role), in line with the customisability rule: derived from the theme, so
  overridable, never a hardcoded colour.

## Implementation

- `frus-widgets/src/chart.rs`: the `STRATA_LABEL_SIZE` constant; `BarChart`'s stacked branch writes
  the `%` share at the centre of each stratum tall enough, when `normalized`.

## Verification

- **Widget** `normalized_bars_label_each_strata_with_its_percentage`: 2 categories × 2 visible series
  = **4** labelled strata at 100% (with no axis: no stray `%` ticks), **0** in absolute mode.
- **Golden** `bar_chart_normalized` regenerated: each column shows its shares (60%/40%, 64%/36%…)
  summing to 100%, white text readable over the fills.
- Widgets 360; goldens 63.

## What's left

- Moving out of the charts domain (a new widget: an advanced `Calendar`/`DataTable`).
- A **value** label on top of **absolute** stacked bars (parity with plain bars).
