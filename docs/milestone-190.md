# Milestone 190 — Integrated sign-up wizard (end-to-end demo)

## Analysis

Many recent bricks existed only "in the showcase" (isolated goldens): the clickable `Steps`
indicator (182/183), the `Form` + the clickable `ErrorSummary` (180/181), the
`Toast`/`SnackbarQueue`/`ToastHost` notifications (185/188). We had to **prove they assemble into
a real app** — not just side by side in a test, but tied to a state, a navigation and messages.
That is the role of this integration in `frus-demo`.

## Technical decisions

- **A multi-step wizard as a new route.** `Route::Wizard` joins the screen stack (reachable from
  the drawer). The state lives in a few `TodoApp` fields (`wizard_step`, four values,
  `wizard_submitted`) — `#[derive(Default)]` covers initialisation, no construction site to touch.

- **Each brick in its place, linked by messages.**
  - `Steps(["Account","Security","Review"]).current(step).on_tap(Msg::WizardStep)`: the indicator
    **drives** the navigation (a clicked marker → a step jump, milestone 183).
  - `Form` (pure) is **rebuilt on the fly** from the state at each render **and** on submission —
    the same rules, a single source of truth (`wizard_form`). The `matches` cross-field validation
    ties `confirm` to `password`.
  - Field errors only show **after** a first submission (`wizard_submitted`) — the "submitted vs
    editing" state raised in milestone 181, made concrete here.
  - On the Review step, `ErrorSummary::links` turns each error into a **clickable bullet** that
    jumps to the faulty field's step (`wizard_step_of` → `Msg::WizardStep`), linking 181 and 183.
  - On a valid submission, a success **notification** shows through `ToastHost` (bottom centre, a
    fade-in, milestone 188) and the wizard resets.

- **The flow logic is pure and tested.** `reduce` handles
  `WizardStep/Input/Back/Next/Submit`; `Submit` branches on `wizard_form(app).is_valid()` (notify +
  reset, or reveal the errors and go to Review). No hidden state: everything derives from the
  fields.

- **A showcase bonus.** The demo's existing toast ("Saved") now goes through `ToastHost` too.

## Implementation

- `frus-demo/src/lib.rs`: `Route::Wizard` (+ `save_state`/`restore_state`); 5 `Msg`s; the
  `wizard_*` fields; the `reduce` arms; `wizard_form` / `wizard_step_of` / `wizard_input` /
  `wizard_screen`; the drawer entry; the toast rendered through `ToastHost`.
- `goldens.rs`: `wizard_review_errors` (the Review step with the error summary — the real
  assembly).

## Verification

- **Integration** (`wizard_flow_validates_navigates_and_notifies`): the screen renders; an empty
  submission → `submitted`, a jump to Review, no toast; a valid fill-in → an "Account created"
  toast + the wizard reset; step navigation clamped. The 16 existing demo tests stay **green** (17
  in total).
- **Golden** `wizard_review_errors` **inspected**: `Steps` (Review), a clickable "Please fix 2
  errors", the summary, Back / Create account.
- `cargo build -p frus-demo` **clean** (zero warnings).

## What's left

- **Focusing the faulty field** (beyond the step jump): wiring `Command::focus(key)` to a bullet
  click — requires stable focus keys on the `TextInput`s.
- **Per-step validation** (blocking "Next" while the current step is invalid) — an ergonomics
  variant.
- **Password masking** (an obscured `TextInput`) — a separate widget feature.
