# Jalon 229 — Value in each band (absolute stacked bars)

## Analysis

Milestone 227 writes the share (`%`) in each stratum at 100%; milestone 228 the total on top of an
absolute stacked column. The natural counterpart was missing: each stratum's **value**, inside the
segment, in absolute stacked mode — to read the composition without hovering, as the `%` does at
100%.

## Technical decisions

- **One stratum-label path.** The stacked branch now writes a centred label in each stratum tall
  enough, whatever the mode: the **share (`%`)** at 100%, the **raw value** in absolute mode. The same
  threshold (`STRATA_LABEL_SIZE + 4`), the same colour (`on_primary`, readable over a saturated
  background), the same centring. The 100% behaviour (milestone 227) is **unchanged** — only absolute
  mode gains the label.

- **Coexists with the total (milestone 228).** The total on top + the per-stratum value = a complete
  reading (composition **and** sum), without redundancy: the total is above the column, the values
  inside it.

## Implementation

- `frus-widgets/src/chart.rs`: the stacked branch's stratum label moves from "`%` if `normalized`" to
  "`%` if `normalized`, otherwise `format_value(value)`" (the height guard is now common to both
  modes).

## Verification

- **Widget** `stacked_absolute_bars_label_each_strata_with_its_value`: the strata values (`3`, `4`,
  `6`) are present, and the column totals (`7`, `11`) stay on top.
- The existing 100% tests (`normalized_bars_label_each_strata_with_its_percentage`) stay green (the
  normalised behaviour preserved).
- **Golden** `bar_chart_stacked` regenerated: each stratum carries its value (3/2, 7/5, 5/6, 8/4,
  4/3), the total on top.
- Widgets 362; goldens 63.

## What's left

- Moving out of the charts domain (a new widget: an advanced `Calendar`/`DataTable`).
- A per-stratum value for the **stacked areas** (lines) — today on hover only.
