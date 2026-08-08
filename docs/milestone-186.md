# Milestone 186 — DatePicker: dual calendar (long ranges)

## Analysis

Range mode (milestone 184) fits within **one** month. But a range often crosses the boundary —
"28 July to 3 August" — and the user then has to navigate from month to month blind, without
seeing both endpoints together. The established date-range picker shows **two months side by
side**; frus did not have it.

## Technical decisions

- **Composing two `DatePicker::range`s.** `range_dual(year, month, …)` builds the requested month
  **and the next** (with a December → January year rollover), each a `DatePicker::range` sharing
  the **same** `[start, end]` range, then places them in a `Flex::row`. No range logic duplicated:
  the band **crosses** the boundary naturally because `range_mark` compares **full**
  `(year, month, day)` dates (milestone 184).

- **Disambiguating the clicked month.** In single mode, `on_select` yields the **day** (the month
  being the displayed one). In dual mode, `on_select` yields the **full date**
  `(year, month, day)`: each month wraps its internal `on_select(day)` into
  `on_select((its_year, its_month, day))`. The shared callback (`on_select`, `on_nav`) is put in an
  `Rc` to feed both months; `on_nav` shifts the **pair**.

- **A `dual` flag for the width.** `DatePicker` gains a `dual` field: `style()` returns
  `2 × month_width + gap` in dual mode, one month's width otherwise. The rest (grid, cells,
  painting) is **unchanged** — dual mode is just an arrangement of two single calendars.

## Implementation

- `datepicker.rs`: `DatePicker::range_dual` (the month + the next, a shared `Rc`, a `Flex::row`);
  the `dual` field (+ the width in `style`); `assemble` initialises `dual: false`.
- `goldens.rs`: `date_range_dual` (July + August 2026, the range 28/07 → 03/08).

## Verification

- **Unit**: `range_dual_shows_two_consecutive_months` — a single child (the row), two calendars; a
  December 2026 → January 2027 rollover; clicking 3 January in the right-hand month reports
  `(2027, 1, 3)` (the full date, at the expected index). Milestone 184's tests **green**.
- **Golden** `date_range_dual` **inspected**: July (the 28th as the start + 29–31 banded), August
  (1–2 banded, the 3rd as the end) — the range continues across the month boundary.
- `cargo test -p frus-widgets datepicker::` **green**.

## What's left

- **Shared navigation**: each month carries its own ‹ › arrows (four in total); a single
  navigation bar above the pair would be tidier (a layout extension).
- **A hover preview** (a provisional band up to the hovered day during entry) — the hover state
  already exists, to be wired app-side.
