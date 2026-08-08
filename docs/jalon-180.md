# Jalon 180 — Forms: cross-field validation & error summary

## Analysis

The `form` module validated each field **in isolation** (a `Rule` only sees one value) and
already exposed `is_valid` / `error(key)` / `first_invalid()` (enough to focus the first faulty
field). Two common needs were missing: (1) **cross-field validation** (a field compared to
another — "confirm password", "end date ≥ start"); (2) an **error summary** at the top of the
form after an invalid submission.

## Technical decisions

- **Remembered values → cross-field validation.** `Form` now retains each field's **value**.
  `field_with(key, value, |value, form| …)` validates through a function that receives the
  **partial** form (the fields already declared) and can consult `form.value(other)`.
  `matches(key, value, other, message)` is the shorthand for it (strict equality —
  confirmation). The referenced field must be declared **first** (single-pass validation, in
  declaration order).

- **`errors()` + the `ErrorSummary` widget.** `Form::errors()` returns `(key, message)` in
  order. The `ErrorSummary::new(messages)` widget turns that into an **error-tinted card** (a
  "Please fix N error(s)" title + one bullet per message), **inert** (no clicks), with
  `is_empty()` so nothing shows when everything is valid. The `form` module stays **pure** for
  the logic; only `ErrorSummary` draws (a dedicated widget).

- **Focusing the first invalid: already tooled.** `first_invalid()` gives the key to focus; the
  application passes it to `Command::focus(key)` on submission (the shell resolves the key
  against the tree — the existing focus milestone). Nothing to add framework-side.

## Implementation

- `form.rs`: `Form` stores `(key, value, error)`; `field_with` / `matches` / `value` / `errors`;
  the `ErrorSummary` widget (a `surface.lerp(error)` background + a border, lines of text).
- `lib.rs`: `pub use form::ErrorSummary`.
- `goldens.rs`: `form_error_summary` (a summary above a field in error).

## Verification

- **Unit**: `cross_field_confirm_password` (`matches` + `field_with` with `form.value`);
  `errors_lists_all_messages_in_order` (valid ones omitted, order preserved);
  `error_summary_lists_messages` (a title + one bullet per message, empty → `is_empty`). The
  existing tests (`field`, `first_invalid`) and the module's doctest stay **green**.
- **Golden** `form_error_summary` **inspected**: a "Please fix 2 errors" card + bullets, above
  the Email field in error — no regression.
- `cargo test --workspace` **green**.

## What's left

- A **clickable summary**: clicking a bullet to focus the corresponding field (the
  `ErrorSummary` would carry a message per item) — an extension.
- **Rich inter-field rules** beyond equality (multiple dependencies): already covered by
  `field_with`, to be documented through recipes.
