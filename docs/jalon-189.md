# Jalon 189 — DateTimeRange: date + time range

## Analysis

We can pick a **date** range (the dual calendar, milestone 186) and a **time** range (`TimeRange`,
milestone 187), but not both together. Booking a real slot — "from 28 July 09:00 to 3 August
17:30" — needs a single screen combining the date **and** the time of both start and end. It is
the "range" counterpart of
[`DateTimePicker`](../crates/frus-widgets/src/datetimepicker.rs).

## Technical decisions

- **Pure composition, like `DateTimePicker`.** `DateTimeRange` stacks the dual calendar
  ([`DatePicker::range_dual`]) and the time range ([`TimeRange`]) in a column, topped by a
  **summary** "start → end". No new logic: each brick keeps its own, the composite merely
  **forwards** the two message channels.

- **Two distinct channels, each true to its nature.** **Dates** go through
  `on_date((year, month, day))` (the application decides which endpoint receives the clicked day —
  as in the date range) and `on_nav(±1)`; **times** through `on_time(endpoint, field, value)`
  (where the endpoints are explicit, Start/End). The date range crosses the month boundary
  naturally (dates compared in full, milestone 184).

- **A conditional summary.** The "July 28, 2026 09:00 → August 3, 2026 17:30" line only appears
  once **both** dates are set (the selection complete); otherwise the composite reduces to
  `[calendar, times]` — the same rule as `DateTimePicker`'s summary.

## Implementation

- `datetimerange.rs`: `DateTimeRange<Msg>` (`new`, composing `range_dual` + `TimeRange`, the
  conditional summary); `impl Widget` (a column, with no painting of its own).
- `lib.rs`: `mod datetimerange` + `pub use datetimerange::DateTimeRange`.
- `goldens.rs`: `datetime_range`.

## Verification

- **Unit**: `summary_appears_only_with_both_dates` (0/1 endpoint → `[calendar, times]`; 2
  endpoints → `[summary, calendar, times]`); `renders_the_combined_summary` (the exact text
  "July 28, 2026 09:00 → August 3, 2026 17:30").
- **Golden** `datetime_range` **inspected**: the summary at the top, the dual calendar (the range
  28/07 → 03/08 crossing the month), the time range Start 09:00 / End 17:30.
- `cargo test -p frus-widgets datetimerange::` **green**.

## What's left

- A built-in **end ≥ start constraint** (dates and times) — for now the application's
  responsibility.
- A derived **duration** shown in the summary ("6 d 8 h 30") — a presentation extension.
