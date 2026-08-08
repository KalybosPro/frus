# Jalon 184 — DatePicker: selecting a date range

## Analysis

`DatePicker` (our own monthly calendar) only handled a **single date**
(`selected: Option<u32>`). Booking a stay, filtering a "from…to…" report, planning leave: all need
a start→end **range**. The established date-range picker highlights the two endpoints and
**shades the interval**. frus did not have one.

## Technical decisions

- **A state marker per cell, no longer a boolean.** The `Day` cell carried `selected: bool`; it
  now carries a `DayMark`: `Off`, `Selected` (single mode, unchanged), `Start`, `End`, `Between`.
  Single-mode rendering (`Off`/`Selected`) is **identical** to the previous milestone — only range
  mode adds states.

- **Endpoints as pills, the interior as a band.** `Start`/`End` keep the selected days' **solid
  pill** (`primary`); `Between` paints a **soft band** (`primary` at 18%, square corners so
  neighbouring days touch). To connect the pill to the band, each **endpoint** paints a
  **half-band** on its inner side (`Start` → right, `End` → left). The band breaks naturally at
  the end of a week (as Material does) — no line-wrapping logic.

- **Date comparison by tuple.** A day's marker comes from
  `range_mark((y, m, d), start, end)`: `(year, month, day)` dates compare
  **lexicographically**, so `<` is chronological order — the interval crosses **month
  boundaries** with no special code. A **pure** function, tested separately.

- **Factored constructors.** `new` (single) and `range` share `assemble(...)`, which builds the
  header, the weekday row and the grid; only `mark_of(day)` differs. `range` stays
  **controlled**: `on_select(day)` reports the clicked day of the displayed month, the application
  decides whether it becomes the start or the end (and handles a lone endpoint mid-selection:
  `end == None` → only the start is marked).

## Implementation

- `datepicker.rs`: `enum DayMark`; `Day.mark` (+ band/half-band/pill painting); the pure
  `range_mark` function; `DatePicker::range` + a shared `assemble` (`new` reduces to it).
- `goldens.rs`: `date_range` (July 2026, the 10th to the 15th).

## Verification

- **Unit**: `range_marks_endpoints_and_interior` (endpoints, interior, outside the range, a month
  crossing, a lone endpoint); `range_builds_grid_with_clickable_days` (a full grid, clickable
  days). The single-calendar tests (`date_math_is_correct`,
  `builds_header_weekdays_and_grid`) **green**.
- **Golden** `date_range` **inspected**: the 10th and 15th as solid pills, 11–14 as a soft band,
  the break at the end of the week.
- `cargo test -p frus-widgets datepicker::` **green**.

## What's left

- **A hover preview** (a provisional band up to the hovered day during selection) — the hover
  state already exists framework-side; to be wired app-side.
- A **dual calendar** (two months side by side) for long ranges — composing two `DatePicker`s.
- **Endpoints outside the displayed month**: already correct (the month's days compare against
  full dates); a "…" marker showing the continuation would be a plus.
