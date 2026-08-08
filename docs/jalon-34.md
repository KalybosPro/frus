# Jalon 34 — New widgets: Avatar, Stepper, Rating

Three more widgets (display / numeric input / rating).

## Widgets

- **`Avatar::new("Ada Lovelace")`** — a round **initials** pill (the first 2
  letters of the words, uppercased), on an accent background (`.color()` to
  override, `.size()`). Pure rendering.
- **`Stepper::new(value, on_change).range(min, max).step(n)`** — a controlled
  **−/value/+** numeric selector. A `[−, text, +]` composite; the buttons emit
  the **clamped value** (the stepper clamps to `[min, max]` itself).
- **`Rating::new(value, max, on_rate)`** — a rating in **clickable stars**;
  clicking the i-th emits `on_rate(i + 1)`. Full stars `primary` / empty ones
  `muted`, focusable (reachable from the keyboard).

## Demo (integration)

- An **`Avatar`** (initials) to the left of each task row.
- A **`Rating`** ("Your feedback") and a **`Stepper`** ("Quantity") in the
  Settings controls card.

## Tests

- `Avatar`: 2 uppercase initials; paints a circle + the initial.
- `Stepper`: `+`/`−` emit the value ±step, **clamped** to the range.
- `Rating`: `max` stars; clicking the i-th → `on_rate(i+1)`; full ≠ empty
  (colour).
- 68 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Stepper`: the text width varies (the buttons shift slightly with the value).
- `Rating`: no half stars; no hover "preview".
- `Avatar`: initials or a solid colour (no image — there is no image widget yet).
