# Jalon 27 — DX / ergonomics (writing a UI faster)

A priority set from the top: **development must be very easy and fast**. So we
tackle writing ergonomics before piling on features. Purely **additive** — the
constructors remain available.

## What is added

Layout macros and shortcut functions (the `dsl` module):

```rust
// Before
Flex::row()
    .align(Align::Center)
    .gap(12.0)
    .child(Text::new("Name").size(18.0))
    .child(Flex::row().flex(1.0))
    .child(Button::new("Add").on_press(Msg::Add))

// After
row![
    text("Name").size(18.0),
    spacer(),
    button("Add", Msg::Add),
]
.align(Align::Center)
.gap(12.0)
```

- **`row![a, b, c]` / `column![a, b, c]`** → a `Flex` with those children, still
  **chainable** (`.gap()`, `.align()`, `.padding()`…). `row![]` = an empty
  container.
- **`text(s)`** = `Text::new(s)`.
- **`spacer()`** = a flexible spacer (`Flex::…flex(1.0)`), which pushes its
  neighbours apart.
- **`button(label, msg)`** = `Button::new(label).on_press(msg)` (chainable:
  `.variant()`, `.size()`).

## Proof: the demo refactored

Every todo view (`todo_screen`, `settings_screen`, `todo_row`,
`confirm_content`) rewritten with the DSL: a **flatter, more readable** tree,
with less noise (cascading `.child(...)` → `[...]` lists). No regression (same
tests, identical stopwatch and rendering).

## Technical decision (alternatives)

- **Macros + helpers** rather than a generalised **`Element`** (accepting
  `.child("string")`). The same readability gain, with **zero breakage** of the
  `Widget` trait. An `Element` newtype (the iced way) would force `children()` to
  change everywhere plus a trait-coherence nightmare; kept as an evolution if the
  need is confirmed.

## Tests

- `dsl`: `row!`/`column!` produce the right number of children (including nesting
  and an empty row); `button(label, msg)` does emit the message.
- Non-regression: the whole suite (widgets, shell, demo) + the on-screen demo +
  the stopwatch.

## Next step

The DX-first roadmap is validated: **J28 keyed reconciliation** (correctness +
list DX), then J29 keyboard nav/a11y, J30 window robustness, J31 rich widgets.
The DX reflex is kept at every milestone.
