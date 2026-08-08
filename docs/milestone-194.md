# Milestone 194 — Wizard: revealing the password

## Analysis

The wizard's password fields (milestone 192) were always masked: there was no way to **check**
what you were typing — a source of errors and frustration. The classic "show / hide" toggle was
missing.

## Technical decisions

- **Composition, no new mechanism.** `TextInput::obscure(bool)` already existed (an earlier
  milestone); it is enough to drive its argument from a `wizard_reveal` application state and add a
  toggle button. A single toggle reveals **both** fields of the Security step (password +
  confirmation), which is consistent (you are comparing two entries).

- **A text toggle rather than an eye icon.** The icon set is *filled* (no outlines): a recognisable
  "eye" is expensive there, and an icon **inside** the field would require positional click routing
  shell-side (the `Widget` trait exposes no positional click). A "Show password" / "Hide password"
  button below the fields is clear, offers a large target, and stays 100% composable.

## Implementation

- `frus-demo/src/lib.rs`: the `wizard_reveal` state; `Msg::WizardToggleReveal` (+ a `reduce` arm);
  the Security step passes `obscure = !wizard_reveal` to both fields and adds the toggle button.
- `goldens.rs`: `wizard_password_revealed` (visible passwords + "Hide password").

## Verification

- **Golden** `wizard_password_revealed` **inspected**: "secret12" readable in both fields, a "Hide
  password" button. (The masked state is still covered by `wizard_password_step`, milestone 192.)
- The 18 demo tests stay **green**; `cargo build -p frus-demo` **clean**.

## What's left

- **An eye icon in the field** (a clickable `suffix_icon`): requires an outline eye icon and
  positional click routing for the suffix shell-side — a separate framework extension.
