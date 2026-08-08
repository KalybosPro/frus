# Milestone 39 — Click fix + new widgets: DatePicker, Carousel, Alert

## Critical fix (committed separately: `a82eeae`)

Since J29 (focusable buttons), the press handler had been setting a
`Drag::TextSelect` on **any** focused widget (code written for text fields
only); on release, that spurious drag was consumed **before** the dispatch → so
**mouse clicks were being swallowed** (the keyboard still worked). The fix: only
start a selection if `cursor_at()` returns `Some` (the invariant: `TextInput` →
`Some`, buttons → `None`). Reported by the user while testing the real app.

## Widgets

- **`DatePicker::new(year, month, day, on_select, on_nav)`** — a controlled
  monthly calendar, built on `Grid` (7 columns). A "‹ Month Year ›" header,
  weekday names, and the grid of days (the selected cell brought forward, empty
  cells before the 1st). **Hand-rolled date maths**: leap years, days per month,
  weekday (Sakamoto) — no time dependency.
- **`Carousel::new(index, total, on_change, current_slide)`** — ‹ › arrows
  (disabled at the bounds) around the current slide **supplied by the app** (only
  one is realised). `on_change(index∓1)`.
- **`Alert::new("text").title("...").warning()`** — a **persistent** message box
  (Info/Success/Warning/Error: tinted background + accent bar + glyph), to be
  distinguished from the transient `Toast`.

## Demo

- An `Alert` ("Tip") at the top of the todo card.
- A `DatePicker` in the Settings controls card (year/month/day state + month
  navigation).
- A `Carousel` (3 slides) in the "About" tab.

## Tests

- The fix: `only_text_inputs_place_a_cursor` (Button `cursor_at` = None,
  TextInput = Some).
- `DatePicker`: date maths (leap years, weekday), 3 children (header / weekdays /
  grid), cell count = empty cells + days in the month.
- `Carousel`: bounded arrows; ‹/› → `on_change`.
- `Alert`: variant → accent bar + title + text painted.
- 85 frus-widgets tests.

## Limits (v1)

- `DatePicker`: a single day selected (no range); no keyboard date entry.
- `Alert`: single-line text (no automatic wrapping).
