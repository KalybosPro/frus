# Milestone 202 — Eye icon + password reveal inside the field

## Analysis

Milestone 198 made a `TextInput`'s **suffix** icon clickable (`on_suffix`), and noted that an
(outline) **eye icon** was missing to reveal a password *inside* the field — the gesture expected
everywhere. The sign-up wizard did reveal passwords, but through a "Show / Hide password" button
**next to** the fields. We bring the action back into the field, with the proper icon.

## Technical decisions

- **Two icons, `Eye` and `EyeOff`** (the usual visibility / visibility-off pair). The icon set is
  **filled** (the non-zero rule); an eye, though, is a hollow **ring** with a pupil. We get it
  without changing the engine: an outer contour (the almond) + an inner contour **traced in
  reverse** (the opposite winding → the opening cancels to 0 = transparent) + a solid pupil (a
  non-zero winding at the centre). The opening is therefore guaranteed **whatever** the absolute
  drawing direction. `EyeOff` adds a diagonal bar (hidden).

- **Revealing inside the field.** `wizard_input` takes an `eye: Option<bool>` parameter:
  `Some(revealed)` places the suffix icon (`EyeOff` if revealed, otherwise `Eye`) and
  `on_suffix(WizardToggleReveal)`. Both of the wizard's password fields use it; the external "Show /
  Hide" button goes away. Masking (`obscure`) is still driven by `wizard_reveal`, the icon only
  **toggles** that state.

## Implementation

- `frus-widgets/src/icons.rs`: the `IconName::{Eye, EyeOff}` variants + `eye(off)` (the opposite
  ring + the pupil, an optional bar); the `push_verb` helper to copy the pupil's circle.
- `frus-demo/src/lib.rs`: `wizard_input` gains `eye: Option<bool>` (the suffix icon + `on_suffix`);
  the "Security" step passes `Some(app.wizard_reveal)` to both fields and loses the external button.
- `frus-test/tests/goldens.rs`: the `password_eye` golden (a masked field + the suffix eye).

## Verification

- **Unit** (`eye_is_a_ring_with_a_pupil_and_off_adds_a_slash`): `Eye` = 3 closed subpaths (two
  almonds + the pupil); `EyeOff` = 4 (with the diagonal); both appear in the "every icon produces a
  non-empty path" test.
- **Golden** `password_eye`: a masked "Password" field (dots) with the eye icon on the right.
- **Manual**: on the Security step, the eye in the field reveals / masks both passwords.

## What's left

- **Hovering the suffix** (a hand cursor, highlighting the eye); an eye icon in password fields
  outside the wizard (sign-in, settings).
