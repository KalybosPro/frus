# Jalon 238 — Demo: filtered calendar (weekends greyed out)

## Analysis

`DatePicker::filtered` (milestone 235) allows days to be disabled by predicate, but the demo was still
using the plain calendar. This milestone **anchors it in the app** with the most telling case: a
"Weekdays only" toggle that greys out **weekends**.

## Technical decisions

- **A controlled `Switch` toggle.** A `weekdays_only` state driven by `Msg::SetWeekdaysOnly`; the
  showcase (the Settings route) shows the switch above the calendar.

- **`demo_calendar(app)`.** Returns `DatePicker::filtered(..., |(y,m,d)| !is_weekend(y,m,d), ...)`
  when the toggle is on, otherwise `DatePicker::new(...)`. It demonstrates the predicate on real data,
  without touching the widget.

- **Our own weekend computation.** `weekday` (Sakamoto, 0 = Sunday) + `is_weekend`
  (Saturday/Sunday) in the demo — no time dependency, consistent with the `DatePicker`'s spirit.

## Implementation

- `frus-demo/src/lib.rs`: the `weekdays_only` state + `Msg::SetWeekdaysOnly` + the reduce arm; the
  `weekday`/`is_weekend` helpers; `demo_calendar`; the `Switch` + the conditional calendar in
  `settings_screen`.

## Verification

- **Demo** `calendar_weekdays_only_filters_weekends`: `is_weekend` correct (4–5 July 2026 = a weekend,
  the 6th = a Monday); the toggle sets `weekdays_only`, the showcase renders filtered then unfiltered.
- Demo 34; the workspace (shell) compiles; the widgets/goldens unchanged.

## What's left

- A "selected row" state on the data screen (`on_select_row`).
- A **custom** sort key per `DataTable` column (dates, formatted amounts).
