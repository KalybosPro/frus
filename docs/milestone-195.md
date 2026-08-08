# Milestone 195 — Steps: "completed" state driven by validity

## Analysis

`Steps` (milestones 182–183) marked a step "completed" (a tick) **by position alone**
(`i < current`). But in a guarded wizard (milestone 192, "Next" blocked while a step is invalid), a
*past* step can become **invalid** again (going back + a free jump through `on_tap`): the indicator
then lied by showing it ticked. Steps had to be marked by **validity**, not by position.

## Technical decisions

- **An explicit, optional "completed" mask.** `Steps::completed([bool, …])` sets, per step, whether
  it is done — typically the validity computed by the `Form`. Without that call, the default
  `i < current` rule holds: **every existing use and its goldens are unchanged** (backwards
  compatible).

- **A single decision point.** All the painting (a tick vs a number, a crossed connector or not)
  goes through `is_done(i)`: the mask if one is supplied, `i < current` otherwise. The **current
  step** always shows its **number** (even when valid) — we only tick the *other* completed steps,
  as Material's stepper does.

## Implementation

- `steps.rs`: the `completed` field (+ the `completed` builder); `is_done` (in the unbounded
  `impl<Msg>` block, called from `paint`); the connector and the marker use `is_done` instead of
  `i < current`.
- `frus-demo/src/lib.rs`: the wizard passes `.completed([valid_0, valid_1, all_valid])` (the same
  predicates as the "Next" guard).

## Verification

- **Unit**: `completed_mask_overrides_position` — with no mask, `is_done = i < current`; with a
  mask, independent of position (an invalid step 0 not ticked despite `i < current`, a valid step 2
  ticked despite `i > current`); a shorter mask → the missing ones not completed. Milestones
  182–183's tests stay **green**.
- **Golden** `wizard_password_revealed` (milestone 194) **inspected**: the Account step shows
  **ticked** through `completed`, the current step (Security) as a number. The `form_wizard` /
  `wizard_*` goldens (without `completed`) are **unchanged**.

## What's left

- **Locking jumps to a step not yet reached** (beyond the visual marking) — the "Next" guard covers
  sequential progress, but `on_tap` still allows free jumping.
