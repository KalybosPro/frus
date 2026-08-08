# Jalon 181 — Forms: clickable error summary

## Analysis

The `ErrorSummary` widget (milestone 180) listed a form's errors but stayed **inert**: a user
seeing "• Enter a valid email address" could not click it to jump to the faulty field — they had
to find it by hand. On a long form, the summary should act as a **table of contents**: clicking a
bullet focuses the field.

## Technical decisions

- **Bullets = widgets, no longer `Text`.** Each bullet becomes a small (private) `Bullet` widget:
  the same rendering as before (`on_surface` text on the tinted card) but carrying an `on_click`.
  `ErrorSummary::new(messages)` keeps the bullets **inert** (`message: None`);
  `ErrorSummary::links([(message, msg), …])` makes them **clickable** bullets emitting `msg` —
  typically a `Msg::FocusField(key)` that the application turns into `Command::focus(key)`. Both
  constructors share `assemble()` (the "Please fix N error(s)" title + the bullets).

- **Clickable = focusable + highlighted.** A clickable bullet is `focusable()` and exposes
  `Role::Button` semantics (keyboard navigation + screen readers); it paints a discreet highlight
  (`error.fade(0.12)`) driven by `status.hover_progress`/`focus_progress`. An inert bullet is
  **neither** focusable **nor** clickable — a purely informational summary stays identical to
  milestone 180 (the golden unchanged to the eye).

- **The bullet → field link stays app-side.** The framework does not "know" the fields: the
  application supplies the `Msg` per bullet (often `Form::errors()` zipped with the keys) and
  focuses through the existing focus mechanism. `ErrorSummary` stays a presentation widget.

## Implementation

- `form.rs`: `ErrorSummary::links` + `assemble()`; the private `Bullet { label, message }` widget
  (a full-width `style`, `paint` for the text + a conditional highlight, `on_click`, `focusable`,
  `semantics`).

## Verification

- **Unit**: `error_summary_links_emit_focus_messages` — the title is not clickable, each bullet
  emits its `Msg` in order and is focusable; the `new` variant stays inert (no click, no focus).
  The existing tests (`error_summary_lists_messages`, validation) **green**.
- **Golden** `form_error_summary` regenerated and **inspected**: a "Please fix 2 errors" card +
  two clear bullets above the Email field — no visual regression.
- `cargo test -p frus-widgets form::` **green**.

## What's left

- **A "submitted" vs "editing" state**: only showing the summary / the errors after a first
  submission — purely app-driven (a `bool submitted`), to be documented as a recipe rather than
  coded into the pure framework.
- **Bullet → field highlight** (beyond the focus): the application can also briefly tint the
  targeted field (already possible through the interaction state).
