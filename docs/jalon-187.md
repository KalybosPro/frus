# Jalon 187 — TimePicker: time range (start → end slot)

## Analysis

`TimePicker` picks **one** time. Booking a slot, setting opening hours, scheduling a meeting: all
call for **two** times — a start and an end. It is the temporal counterpart of the dual calendar
(milestone 186); frus did not have it.

## Technical decisions

- **Composing two `TimePicker`s.** `TimeRange` places two pickers labelled "Start" and "End" side
  by side (each a `[label, TimePicker]` column in a `Flex::row`). All the logic (24 h/12 h grids,
  the preview, the minute step) is **reused** as is — no duplication.

- **A single tagged callback.** Rather than four closures (hour/minute × start/end), `TimeRange`
  takes **one** `on_change(Endpoint, TimeField, u32)`: each internal `TimePicker` wraps its
  `on_hour`/`on_minute` to prefix the **endpoint** (`Start`/`End`) and the **field**
  (`Hour`/`Minute`). The callback is put in an `Rc` to feed both pickers; the values stay in
  **24-hour** form (as in `TimePicker`). The application receives a single message and decides how
  to update its state (and, if needed, to constrain end ≥ start — app-side logic).

- **Options propagated.** `hour12()` and `minute_step(n)` apply to **both** pickers through
  `rebuild` (the same settings on either side).

## Implementation

- `timepicker.rs`: `enum Endpoint { Start, End }`, `enum TimeField { Hour, Minute }`;
  `TimeRange<Msg>` (`new`/`hour12`/`minute_step`, `rebuild` builds the two tagged columns, an `Rc`
  for the shared callback); `impl Widget` (a row, with no painting of its own).
- `lib.rs`: `pub use timepicker::{Endpoint, TimeField, TimeRange}`.
- `goldens.rs`: `time_range` (Start 09:00 / End 17:30, minutes in steps of 15).

## Verification

- **Unit**: `range_builds_start_and_end_pickers` (two columns; a 15-minute step → 4 cells;
  clicking 09 h on the End side emits `Set(End, Hour, 9)`); `hour12_applies_to_both_pickers` (the
  12-hour hours section = a label + AM/PM + the grid on both sides). The existing `TimePicker`
  tests **green**.
- **Golden** `time_range` **inspected**: "Start" 09:00 (hour 09 + minute 00 highlighted), "End"
  17:30 (hour 17 + minute 30), minutes 00/15/30/45.
- `cargo test -p frus-widgets timepicker::` **green**.

## What's left

- A built-in **end ≥ start constraint** (greying out earlier hours on the End side) — for now the
  application's responsibility.
- A derived **duration** shown between the two (e.g. "8 h 30") — a presentation extension.
