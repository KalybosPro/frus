# Jalon 132 — Decorated form field (label, hint, help, error)

## Analysis

All three platforms exist (desktop, Android, Web); time to come back to the **product**.
The first brick of a real application is the **form** — and the `TextInput` field was
bare: a value, `on_input`, `on_submit`, and nothing else. No label, no hint, no help
text, and above all **no error state**. A real form needs to announce each field and to
signal invalid input visually.

The established shape solves this with one field widget carrying a decoration: a
**single** widget holds the label, the hint, the help text and the error. We adopt that
shape.

## Technical decisions

- **One widget, decorated — not a separate `FormField`.** We enrich `TextInput` rather
  than stacking a parent widget on top. All the editing logic (caret, selection, IME,
  horizontal scrolling) is **reused as is**; only the layout adds a label line above and
  a help/error line below.

- **The input box is a sub-rectangle.** `style()` now reserves
  `label_block + field_height + sub_block` in height; `paint()` computes the `field` box
  (a vertical inset) and anchors the border, the text, the caret and the selection to it.
  The box takes the full **width** — so the horizontal hit-test (`cursor_at`,
  single-line) is unchanged: clicking to place the caret stays exact, without a line of
  code touched.

- **Validity belongs to the application.** The field evaluates nothing: it **displays**
  the result. `error(msg)` switches the border and label to the theme's error colour and
  shows `msg` under the field (the error hides the help). In the Elm architecture, the app
  computes the error as a pure function of its state and passes it to the `view` — no
  global key, no mutable form state.

- **Customisable, theme tokens.** The colours come from the `Theme` (`error`, `muted`,
  `border`, `focus`): nothing is hardcoded, it is consistent in light/dark, and it is
  overridable through the theme — in line with the "customisable" rule.

- **Accessibility.** The field's semantics carry the label (and the error, concatenated)
  as its `label`, so screen readers announce "Email, Enter a valid email address".

## Implementation

- `crates/frus-widgets/src/textinput.rs`: the `label`/`placeholder`/`helper`/`error`
  fields and their builders; the `label_block`/`sub_block`/`field_height` metrics;
  `style()` and `paint()` extended (the label above, the hint when empty, the help/error
  below, the error colours); `semantics()` enriched. Tests: the height growth, the error
  border, the hint shown only when empty.
- `crates/frus-test/tests/goldens.rs`: the `decorated_form` golden — a field in error
  above a field at rest (hint + help), both decoration states frozen.

## Verification

- **Rendered and looked at** (the "render to see" practice): the Email field in error has
  a red label, border and message; the Password field shows its muted hint and its help
  text. Frozen as the `decorated_form.png` golden.
- **Unit**: `cargo test -p frus-widgets textinput` green (including the 3 new ones); no
  regression of the existing editing tests (the full-width box preserves the hit-test).
- **Workspace**: `cargo test --workspace` stays green.

## What's left

- **Derived components**: a multi-line `TextInput`, a password field (masking),
  prefix/suffix icons.
- **Form help**: a grouped validation helper (validate every field, focus the first in
  error) — still pure, app-side.
- An animated **floating label** (at rest inside the box → floating above on focus),
  Material style.
