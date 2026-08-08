# Milestone 182 — Multi-step form: `Steps` indicator

## Analysis

A long form is split into **steps** (a wizard): account, profile, review… The user needs to know
**where they are** — which steps are done, which one is current, which remain. That is the role of
Material's stepper: a row of linked numbered markers, each completed / current / upcoming. frus
did not have one.

The name `Stepper` is **already taken** by the numeric −/value/+ picker (an earlier milestone); so
the new indicator is called **`Steps`**.

## Technical decisions

- **A purely visual, self-painted widget.** `Steps` has **no children**: it paints the markers,
  connectors, numbers/ticks and labels itself within its `bounds` (markers spread edge to edge,
  `center_x`). No internal state — the current step is a plain `usize` supplied by the
  application. Simple, deterministic, testable at the pixel level.

- **Three readable states.** *Completed* (`i < current`): an **accent** disc + a **tick** (a 16 px
  `Check` icon). *Current* (`i == current`): an accent disc + a light **number**. *Upcoming*: a
  **bordered surface** disc + a muted number. **Crossed** connectors (before the current step)
  take the accent, the others the border colour. A label under each marker, muted outside the
  current step. That is exactly Material's stepper visual grammar.

- **Navigation & validation are app-side.** `Steps` orchestrates nothing: the application holds
  the current step, wires Back/Next buttons, and validates **per step** with a
  [`Form`](../crates/frus-widgets/src/form.rs) (milestones 180–181) — a final summary through
  `ErrorSummary`. The widget stays a **view** of the progress, not a state machine.

- **Customisable.** `current(i)` (clamped to the last index), `color(c)` overrides the accent
  (completed/current markers + crossed connectors); otherwise the theme's `primary`.

## Implementation

- `steps.rs`: `Steps { labels, current, color }`; the `new` / `current` / `color` builders;
  `impl<Msg> Widget<Msg>` (non-generic, like `Icon`) — a full-width `style` × a fixed height,
  `paint` for the connectors then the markers (a `fill_path` tick or a `text` number) then the
  labels.
- `lib.rs`: `mod steps;` + `pub use steps::Steps;`.
- `goldens.rs`: `form_wizard` (a 2/3 indicator + step content + a Back/Next bar).

## Verification

- **Unit**: `current_is_clamped_to_last` (an out-of-range index → the last; an empty list → 0);
  `markers_reflect_progress` (2 completed steps → 2 ticks and no "1"/"2" numbers; the current one
  → "3"; upcoming → "4"; every label drawn).
- **Golden** `form_wizard` **inspected**: "Account" ticked (accent), a green crossed connector to
  the current "Profile" ("2"), a grey connector to the upcoming "Review" (a bordered "3"), the
  labels beneath, then the step title, the field and the Back/Next buttons.
- `cargo test -p frus-widgets steps::` **green**.

## What's left

- **Clickable markers** (`on_tap(|usize| Msg)`) to jump to an already-visited step: would need
  child markers (a hit-test per marker) — an extension.
- **Vertical orientation** (steps stacked with the content unfolding under the current one, the
  stepper's other form) — an extension.
