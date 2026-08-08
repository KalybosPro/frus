# Jalon 135 — Grouped form validation (pure, app-side)

## Analysis

Milestones 132–134 gave the field what it needs to **display** an error (`error(...)`),
but nothing to **decide** one. Every `view` had to recompute its validity by hand. The
logical counterpart was missing: something to validate a **set** of fields, to know
whether everything passes, to retrieve each field's error (to feed `error(...)`) and to
spot the first failing field (to focus).

The usual answer puts that in a mutable form state reached through a global key. In the
Elm architecture, validity is a **pure function of the state**: we provide pure
*combinators*, not an object to mutate.

## Technical decisions

- **Two pure bricks, zero drawing.** `Rule` (a `&str -> Option<String>` rule) and `Form`
  (a report over a set of fields). The module knows neither widget nor GPU; the
  application calls `Form::error(key)` to feed a `TextInput`'s `error(...)`.

- **Composable rules.** Ready-made constructors (`required`, `min_len`, `max_len`,
  `email`) and a `Rule::all([...])` combinator where the **first** failing rule wins —
  the order carries meaning ("required" before "format").

- **An ordered, queryable report.** `Form::field(key, value, rule)` validates the field
  and pushes `(key, error?)` in declaration order. You then query: `is_valid()`,
  `error(key)`, `first_invalid()` (the key of the first failure — to focus or to
  highlight).

- **`&'static str` keys.** Stable, readable field identifiers with no allocation; the
  application ties each key to its state and its widget.

- **`email` = a heuristic, not the RFC.** `local@domain`, a non-empty local part, a domain
  with at least one dot and no empty label. Enough for an input field, without the trap of
  an RFC 5322 regex.

## Implementation

- `crates/frus-widgets/src/form.rs`: `Rule` (+ constructors, `all`), `is_email`, `Form`
  (`field`/`is_valid`/`error`/`first_invalid`). Tests for the rules, the combinator, the
  report, and a usage doctest.
- `crates/frus-widgets/src/lib.rs`: `pub mod form;` (namespaced access
  `form::{Rule, Form}` — names too generic for the crate root).
- `crates/frus-test/tests/goldens.rs`: the `validated_signup_form` golden — a `Form`
  validates typed values and **drives each field's `error(...)`**, rendered after an
  invalid submission (end-to-end for milestones 132→135).

## Verification

- **End-to-end, looked at**: "ada" triggers "Enter a valid email address" (non-empty but
  not an email → `all` returns the 2nd rule); the masked password "short" triggers "At
  least 8 characters". Both fields in red, labels floated. Frozen as the
  `validated_signup_form.png` golden.
- **Unit + doctest**: the rules (blank, lengths, email), `all` (the first error), the
  report (`is_valid`/`error`/`first_invalid`), an empty form is valid.
- **Suites**: `frus-widgets` + `frus-test` green.

## What's left

- **Focusing the first invalid field**: `first_invalid()` gives the key; commanding the
  focus from the application (mapping key → `WidgetId`) is still to be wired shell-side.
- Extra rules as needed (numeric, range, matching two fields for "confirm password").
