# Jalon 134 — Animated floating label (Material style)

## Analysis

Since milestone 132, the label was **static**, always above the field. Material does
better (and it is the default behaviour of the shape we are following): at rest the label
occupies the box like a hint; as soon as you focus or type, it **floats** upwards and
shrinks. A single element plays both label *and* hint, and the transition guides the eye.

## Technical decisions

- **Reuse the focus animation, do not create one.** The border already animated on
  `status.focus_progress` (interpolated 0→1 by the runtime). The float rides on it: no new
  animation plumbing.

- **Two distinct drivers, position and colour.**
  - **Position / size** follow the *float* `t = field filled ? 1 : focus_progress`. Filled,
    the label stays floated even without focus (the content occupies the box); empty, it
    follows the focus. Every real transition is smooth, because `focus_progress` is
    already 1 while editing.
  - **Colour** follows the *focus* (`focus_progress`) alone: a filled but unfocused field
    keeps a **muted** label (not yet accented) — accented only on focus. (In error, the
    error colour wins at all times.)

- **The height does not move.** `style()` still reserves the label band above the box;
  only the label's **drawing** interpolates between its rest position (inside the box, at
  text size) and its floated position (above, shrunk). The box itself stays fixed — no
  layout jump.

- **The hint yields to the label at rest.** When a label is present, the hint
  (`placeholder`) only **fades in** once the label has floated
  (`α = opacity × focus_progress`): otherwise the two would overlap in the box. With no
  label, the hint shows as before.

## Implementation

- `crates/frus-widgets/src/textinput.rs`: `paint()` — the label interpolates rest↔floated
  (position, size, colour) from `float_t` / `fp`; the hint fades with `fp` when a label is
  present. The `floating_label_rests_in_box_then_floats_up` test (big/low at rest →
  small/high focused).
- `crates/frus-test/tests/goldens/decorated_form.png`: regenerated — the Password field
  (empty, unfocused) now shows its label **at rest inside the box** (the `password_field`
  golden is unchanged: filled ⇒ the label is already floated, muted).

## Verification

- **Rendered and looked at**: at rest, "Password" occupies the box; "Email" (filled) stays
  floated above. Frozen in the regenerated `decorated_form` golden.
- **Unit**: the label is bigger and lower at rest, smaller and higher once focused.
- **Suites**: `frus-widgets` (238) + `frus-test` green, `password_field` unchanged.

## What's left

- A **label notch** (the floated label "cuts" the border, outlined style) — a visual
  refinement.
- An explicit always/never floating behaviour, should a design want to force the state.
- **Grouped validation** (the next milestone).
