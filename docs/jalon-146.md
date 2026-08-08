# Jalon 146 — Time picker (`TimePicker`)

## Analysis

The `DatePicker` (a monthly calendar) already existed, but there was nothing to pick a
**time**. The established counterpart is a clock-dial picker; frus had none. We complete
the date/time family with a time picker consistent with the calendar.

## Technical decisions

- **Grids rather than a dial.** The Material dial (a hand, an arc) is heavy to paint and
  poor at the keyboard. We settle on two **grids of cells** — hours `0–23`, minutes in
  **steps of 5** — in the same visual spirit as the `DatePicker`'s day cells (the same
  `TimeCell` highlighted in `primary`, hover through the state layer, rounded corners).
  Simple, readable, clickable, and already accessible to the pointer.

- **Controlled, like everything else.**
  `TimePicker::new(hour, minute, on_hour, on_minute)`: the displayed time comes from the
  application state; the widget **emits** `on_hour(h)` / `on_minute(m)` on click and decides
  nothing. The `HH:MM` preview reflects `hour`/`minute` **exactly**, even when the minute is
  not a multiple of 5 (no cell is lit then, but the preview stays accurate).

- **Composite, stateless.** Like the `DatePicker`, the picker is just an assembly of
  `[preview, hours section, minutes section]` (each section = a label + a `Grid`), built in
  the constructor. No time logic: the minute step is a plain `MINUTE_STEP` constant.

## Implementation

- `timepicker.rs` (new): `TimeCell<Msg>` (a clickable, highlightable cell);
  `TimePicker<Msg>` assembling the preview and the two grids.
- `lib.rs`: `mod timepicker;` + `pub use timepicker::TimePicker;`.
- `goldens.rs`: the `time_picker` golden (9:30 → hour `09` and minute `30` highlighted).

## Verification

- **Unit**: the `[preview, hours(24 cells), minutes(12 cells)]` structure; the `09:30`
  preview is painted and a selected cell is highlighted in `primary`; a click on a cell does
  emit `Hour`/`Minute` (through `ui.hit` + `ui.msg_for`).
- **Golden** `time_picker` rendered and **inspected**: the `09:30` preview, `09` and `30`
  lit. `cargo test --workspace` green, no existing golden moved.

## What's left

- **Minute-precise minutes** (today a step of 5) and a **12-hour format (AM/PM)**.
- An optional **dial** and direct **keyboard entry** (`HH:MM`), Material 3 style.
- **Combining date + time** in a single flow.
