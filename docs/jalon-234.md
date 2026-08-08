# Jalon 234 — Bounded-range DatePicker (range + [min, max] window)

## Analysis

Milestone 231 bounded the **single** calendar (`bounded`). **Range** mode (`range`) still left every
day clickable — there was no way to impose an entry window on an interval selection (booking a range
inside an open period, say). This milestone closes the parity gap: a **bounded** range mode.

## Technical decisions

- **`range_bounded(start, end, min, max)`.** A superset of `range`: the same range marks (endpoints
  as pills, the days between as a soft band) **plus** milestone 231's `enabled` predicate — a day is
  clickable iff `min <= date <= max` (optional, inclusive bounds). No new infrastructure: it reuses
  `assemble(enabled)` and `range_mark`.

- **A clean combination.** Highlighting (the selected range) and enabling (the allowed window) are
  **orthogonal**: a day can be "between" the range's endpoints while being outside the window (hence
  disabled), and vice versa. Each is computed independently.

## Implementation

- `frus-widgets/src/datepicker.rs`: the new `range_bounded` constructor (mark = `range_mark`,
  enabled = the `[min, max]` test).

## Verification

- **Widget** `range_bounded_disables_days_outside_the_window`: a 10–15 range inside an `[8, 20]`
  window → the 7th and the 21st not clickable, the 8th/12th/20th clickable (including the 12th, a day
  "between", correctly active).
- **Golden** `date_range_bounded`: the 8–20 window active, the 10–15 range highlighted, outside the
  window muted.
- Widgets 370; goldens 67 (`date_range`/`date_bounded` unchanged).

## What's left

- Arbitrary **blackout** days (a predicate/set of isolated dates, not just an interval).
- Wiring the bounded calendars into the demo (the date screen).
