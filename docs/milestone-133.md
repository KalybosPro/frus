# Milestone 133 — Password field (masking) + prefix/suffix icons

## Analysis

Milestone 132 gave the field its decoration (label, hint, help, error). Two form needs
were still missing: **masking** a password, and housing an **icon** inside the field
(search, padlock, currency…). They are the direct complements of the decoration:
obscured text, a prefix icon, a suffix icon.

## Technical decisions

- **Mask the display, not the value.** `obscure(true)` changes only the **rendered**
  string: `display()` returns one dot per character, the real value stays in `value`. All
  the editing (insertion, selection, IME, `text_value` for the input context) works on
  the real value. Since the mask keeps the **character count**, the caret, the hit-test
  and the selection stay aligned index for index — the geometry is simply shaped on the
  masked string.

- **The "show" toggle is composed app-side.** We do not add an *interactive* suffix to
  the field (which would require a sub-hit-test inside the widget). In the Elm
  architecture, the application owns the `show` boolean, renders `.obscure(!show)` and
  places a button next to it that flips that boolean. The field stays pure, with no
  hidden state.

- **Icons = decorative slots, drawn in place.** `prefix_icon`/`suffix_icon` take an
  `IconName`; the field paints its vector path straight into the box (like the `Icon`
  widget), vertically centred, in a muted colour. No child, no nested widget.

- **The icons shrink the content area — everywhere.** `prefix_w`/`suffix_w` reserve
  `ICON_SIZE + ICON_PAD` on each side. The content geometry (origin + text width) is
  recomputed with those insets **both in `paint` and in `cursor_at`**: so a click lands on
  the right index even behind a prefix.

## Implementation

- `crates/frus-widgets/src/textinput.rs`: the `obscure`/`prefix`/`suffix` fields +
  builders; `display()` (the masked string); `layout()` shapes the display;
  `prefix_w`/`suffix_w`; `paint()` draws the icons and inserts the content between them,
  rendering `display()`; `cursor_at()` applies the same insets. Tests: masking (the value
  does not leak, dots drawn, `text_value` intact) and the prefix (path drawn + shifted
  hit-test).
- `crates/frus-test/tests/goldens.rs`: the `password_field` golden (a masked value +
  prefix and suffix icons).

## Verification

- **Rendered and looked at**: a label, a prefix icon, a masked `•••••••`, a suffix icon,
  help — frozen as the `password_field.png` golden.
- **Unit**: `cargo test -p frus-widgets textinput` green (17 tests, 2 of them new); no
  regression of the existing hit-test (with no icon, `left == PAD_X`, identical
  geometry).
- **Suites**: `frus-widgets` + `frus-test` green.

## What's left

- A **built-in visibility toggle** (an interactive suffix): would require a sub-hit-test
  inside the widget — deferred; the app composes it today.
- A **colourable / resizable icon** per field (today: muted, `ICON_SIZE`).
- An animated **floating label** (the next milestone) and **grouped validation** (the one
  after).
