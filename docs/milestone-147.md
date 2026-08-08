# Milestone 147 — Date + time flow, fine-grained minutes & 12-hour AM/PM

## Analysis

The `DatePicker` (calendar) and the `TimePicker` (time) existed separately. The
**combined flow** was missing, and the `TimePicker` was rigid: 24-hour only, minutes frozen
at a step of 5. We round out the date/time family.

## Technical decisions

- **`TimePicker` rebuilds its tree.** Like the `Table` (milestone 145), the picker now
  stores its state (`hour`, `minute`, callbacks) + its options and **regenerates** its
  children (`rebuild`): that opens the way to fluent settings (`hour12`, `minute_step`)
  without multiplying constructors.

- **Clean 12-hour, a single 24-hour callback.** In `hour12`, the grid goes from 0–23 to
  **1–12** and an **AM/PM toggle** appears. The widget is still driven by a single **24-hour**
  hour: each 1–12 cell targets the 24-hour hour of the current half, and AM/PM shifts the
  current hour by ±12. So the application only handles one `on_hour(h24)` — the 12↔24
  conversion is internal (`digit12`).

- **An adjustable minute step.** `minute_step(n)` (clamped 1–60) controls the granularity;
  the selection only lights up if the current minute falls on a step (the preview stays
  exact regardless).

- **`DateTimePicker`, purely composite.** It adds no logic: it stacks the `DatePicker` and
  the `TimePicker`, forwards their four callbacks (`on_day`, `on_nav`, `on_hour`,
  `on_minute`), and tops it all with a **summary** "Month day, year HH:MM" — shown only
  once a day is picked. The state (date, time) stays in the application.

## Implementation

- `timepicker.rs`: moved to a `rebuild`; the `hour12()` and `minute_step(n)` options; the
  AM/PM toggle + the 1–12 grid; a 12/24-hour preview; the `digit12` helper.
- `datetimepicker.rs` (new): `DateTimePicker` combining the two sub-pickers + the summary.
- `lib.rs`: `mod datetimepicker;` + the `DateTimePicker` export.
- `goldens.rs`: the `time_picker_12h` and `date_time_picker` goldens.

## Verification

- **Unit**: `minute_step(15)` → 4 minutes; `hour12()` → a grid of 12 + an AM/PM row, a
  `3:05 PM` preview for 15:05; a 24-hour `09:30` preview; a click → a message; the
  `DateTimePicker` only shows the summary once a day is picked and renders
  "July 11, 2026  09:30".
- **Golden**: `time_picker_12h` (PM, hour 3, minute 05 lit) and `date_time_picker` (the
  summary + the calendar on the 11th + the time 09:30) rendered and **inspected**. The
  existing 24-hour golden (`time_picker`) is unchanged. `cargo test --workspace` green.

## What's left

- An optional **clock dial** and **keyboard entry** of `HH:MM` (Material 3).
- **Validating a complete flow** (an "OK/Cancel" button, returning a single `(date, time)`)
  — here the two halves emit independently.
