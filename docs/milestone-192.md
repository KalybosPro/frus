# Milestone 192 — Wizard: per-step validation, programmatic focus, masked passwords

## Analysis

The wizard (milestone 190) let "Next" through on an invalid step, showed passwords **in the
clear**, and its error bullets only **changed step** without landing on the faulty field. Three
ergonomic gaps the framework already knew how to cover — they just had to be **wired**.

## Technical decisions

- **"Next" governed by the step's validity.** `wizard_step_valid(form, step)` queries the (pure)
  `Form`: Account is valid if `name`+`email` pass, Security if `password`+`confirm` pass. The
  "Next" button gets `.enabled(...)` (milestone 191) → **greyed and inert** while the current step
  is incomplete. Validity stays a **pure function of the state**, not a flag to maintain.

- **Masked passwords.** The Security fields pass `.obscure(true)` (already offered by
  `TextInput`): the display becomes dots, the value being edited stays real.

- **Programmatic focus by key.** Each field is wrapped in `keyed(("wizard", i), …)`; clicking a
  summary bullet emits `WizardFocus(step, field)`, which **jumps to the step** then returns
  `Command::focus(("wizard", field))`. The shell resolves the key against the tree
  (`keyed`/`Command::focus` hash the key identically) and puts the caret **in the field** — no
  longer just on the right step. No new mechanism: the framework already knew how to focus by key.

## Implementation

- `frus-demo/src/lib.rs`: `Msg::WizardFocus(usize, u8)` (+ a `reduce` arm → `Command::focus`);
  `wizard_field_of` / `wizard_step_valid`; `wizard_input` gains `obscure` and the `keyed` wrapper;
  "Next" `.enabled(wizard_step_valid(...))`; the summary bullets → `WizardFocus`.
- `goldens.rs`: `wizard_password_step` (the Security step: masked passwords + a disabled "Next").

## Verification

- **Integration** (the `wizard_flow_*` test extended): Account invalid to begin with;
  `WizardFocus` jumps to the step **and** emits a focus request (`!cmd.is_empty()`); the step
  becomes valid once filled in. The 17 demo tests stay **green**.
- **Golden** `wizard_password_step` **inspected**: `Steps` (Security), two fields in dots, an
  active "Back" next to a greyed "Next".
- `cargo build -p frus-demo` **clean**.

## What's left

- **Marking the `Steps` by validity** (and not only by position) — consistent now that "Next" is
  guarded, but free jumping through `Steps::on_tap` can desynchronise it.
- **Revealing the password** (an eye `suffix_icon` toggling `obscure`) — a small UX addition.
