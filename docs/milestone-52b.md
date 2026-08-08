# Milestone 52b — Unified `Scaffold` (Material screen skeleton)

The AppBar (52a) and the navigation bar lived **inside the body**: they scrolled
with the content instead of being fixed chrome. That is the job of a `Scaffold` —
the **central coordinator** of a screen's structure: pinned top bar, scrolling
body, pinned navigation, plus drawer / sheet / FAB.

## A `Scaffold` widget that subsumes navigation

The developer declares **slots**; the Scaffold assembles them and **chooses the
presentation on its own** according to the width (bottom bar when narrow, side
rail when wide) — it absorbs `NavScaffold`. One codebase, with no branching on
mobile vs desktop.

```rust
Scaffold::new(width, height)
    .background(theme.background)
    .app_bar(header)                       // pinned at the top
    .body(section)                         // scrolls between the bars
    .nav(app.section, Msg::SetSection)     // adaptive navigation (rail | bar)
    .destination("✔", "Tasks").badge(n)
    .destination("▦", "Stats").destination("★", "About")
    .end_drawer(menu, app.drawer_open, Msg::ToggleDrawer)
    .bottom_sheet(sheet, app.sheet_open, Msg::ToggleSheet)
    .build()
```

**Assembly:**
- **Compact**: a column `[top bar · body (flex Scroll) · bottom bar]`.
- **Medium/Expanded**: a row `[rail · column[top bar · body]]`.
- The body is wrapped in a `Scroll` that **scrolls** as soon as the content
  exceeds the viewport.
- The drawer (right, modal) and the modal sheet wrap the skeleton as overlays
  (reusing `Drawer` / `BottomSheet`).
- `inset_pad` only wraps a slot when an inset is **non-zero** — otherwise it
  preserves the parent's stretch (without which the bottom bar collapses).

**Insets** are still handled upstream by `view` (milestone 51, which passes safe
dimensions); the Scaffold pins itself inside that viewport. Eventually, once
every route goes through a Scaffold, it will be able to take over the safe area.

## Demo

All the manual assembly (NavScaffold + Drawer + Stack(toast) + Container) is
replaced by **one** `Scaffold`. The body (input / filters / **list**) scrolls as a
whole — the old fixed-height internal `Scroll` disappears. `todo_screen` and
`screen` now return `Box<dyn Widget>`.

## Validation (on the device)

- Top bar **pinned** (under the status bar), bottom bar **pinned** (above the
  system nav) — confirmed by a screenshot.
- **Navigation works**: tapping Tasks / Stats / About switches the section
  (verified through the `[demo] section -> n` log).
- **Scrolling body**: with 40 tasks, a swipe scrolls the list (#0–6 → #8–17)
  while both bars stay fixed.
- Desktop: `frus-widgets` 122 tests, `frus-demo` 15 tests, a warning-free build.

## Known limit → milestone 52c

- The **FAB** (`Scaffold::fab`) is **disabled in the demo**: it is superimposed
  through a full-screen `Stack` layer, and such a top layer **intercepts the
  clicks** of the bottom half of the screen (a limit of `Stack` hit-testing — the
  same symptom threatens a persistent `Toast`). To be fixed with a non-blocking
  overlay before re-enabling the FAB.
- The permanent drawer in Expanded (3 zones) is not carried over here (modal
  drawer only in Scaffold v1).
