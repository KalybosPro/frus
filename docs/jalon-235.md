# Jalon 235 — Blackout days / selection predicate (DatePicker)

## Analysis

Milestones 231/234 disable **intervals** (`[min, max]`). But a real calendar must also remove
**isolated** dates: public holidays, slots already booked, weekends. Rather than multiplying
constructors (one per shape of constraint), this milestone exposes the **general escape hatch** — a
selectable-day predicate, the proven shape for this.

## Technical decisions

- **`filtered(is_enabled)`.** A day `(year, month, day)` is clickable iff `is_enabled(date)`. That one
  constructor covers **everything**: scattered blackouts (`|d| !holidays.contains(&d)`), weekends
  (`|(y,m,d)| weekday(y,m,d) not in {0,6}`), bounds (`min..=max`), or any combination.
  `bounded`/`range_bounded` remain handy shorthands for the common min/max case.

- **Reuses `assemble(enabled)`.** The public `Fn((i32,u32,u32)) -> bool` predicate is adapted to the
  internal `Fn(u32) -> bool` one by `move |day| is_enabled((year, month, day))`. No new
  infrastructure; the disabled rendering is identical (milestone 231).

## Implementation

- `frus-widgets/src/datepicker.rs`: the new `filtered` constructor.

## Verification

- **Widget** `filtered_disables_days_by_predicate`: a `{12, 18}` blackout in July 2026 → the 12th and
  the 18th not clickable, the 1st and the 13th clickable.
- **Golden** `date_blackout`: days 4, 5, 14, 15, 27 muted/disabled, the rest active, the 21st
  selected.
- Widgets 371; goldens 68 (the existing calendars unchanged).

## What's left

- Wiring a filtered calendar into the demo (greyed-out weekends, say).
- A predicate in **range mode** (today `filtered` covers single mode; `range` has `range_bounded` for
  bounds, but not yet an arbitrary predicate).
