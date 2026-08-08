# Jalon 29 — Keyboard navigation / accessibility

Focus only existed on **click**, and only `TextInput` could receive it. This
milestone adds **Tab** navigation, **keyboard activation** of controls, and a
**visible focus ring** — basic keyboard a11y.

## What is added

- **Focusables**: `Button`, `Checkbox` and `Switch` return `focusable() = true`.
  That is **all** a widget has to do to become accessible → a11y almost for free
  (DX). An accessible widget = focusable + `on_click`.
- **Tab / Shift+Tab** (shell): `Ui::focus_next(current, direction)` walks
  `ui.focusables` (already collected in tree order = visual order), wrapping
  around. Works even with **no** initial focus (→ first / last).
- **Activation** (shell): on **Enter / Space**, if the focused widget is
  focusable and has an `on_click`, that message is emitted (button / checkbox /
  switch). Text fields (which have no `on_click`) fall back to editing (Enter =
  submit, Space = a space) — the presence of `on_click` cleanly separates the
  two.
- **Generic focus ring** (`build_ui`): drawn around the focused focusable, in
  `theme.focus`, with an intensity animated by `focus_progress`. A widget that
  handles its own focus suppresses it through `draws_own_focus() = true`
  (`TextInput`, which keeps its border).

## Trait

```rust
fn draws_own_focus(&self) -> bool { false }   // added (default: the generic ring)
```

No new application surface: the existing widgets become keyboard-navigable
**with no change on the app side**.

## Tests

- `Ui::focus_next`: order + wrap-around, first/last with no focus.
- `Button`/`Checkbox`/`Switch` are focusable (through `focusables.len()`).
- The ring: a focused button adds a primitive bordered in `theme.focus`; a
  focused `TextInput` does not (it handles its own).
- 43 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Slider` / `Dropdown` / `RadioGroup`: fine keyboard navigation (arrows)
  deferred; they will become focusable next.
- No ARIA roles or announcements (winit exposes no screen-reader API): a11y =
  **keyboard + visible focus** for now.
