# Milestone 52a — Adaptive AppBar (Material app bar)

The demo's header crammed ~12 controls into a single row; in Compact, the actions
wrapped onto the next line and **collided with the title**. The very symptom of
"not respecting mobile principles": a desktop toolbar pasted onto a phone.

## Principle: one codebase, adaptation by the framework

The developer declares **one** title, an optional `leading` and a list of
**actions** — never saying "this is for mobile / desktop". The new [`AppBar`]
widget decides **on its own**, from the available width, how many actions fit
inline and **folds the rest into a `⋯` overflow menu**. Wide → everything inline;
narrow → overflow. That is the behaviour of a Material AppBar.

```rust
AppBar::new("My Tasks")
    .width(available_width)                 // a size, not a platform
    .leading(button("☰", Msg::ToggleDrawer))
    .overflow(app.actions_open, Msg::ToggleActions)
    .action("Pause", Msg::ToggleTimer)
    .action("Settings →", Msg::Push(Route::Settings))
    // …
    .build()
```

## Mechanics

- Each action's exact width is **measured** (`frus_text::measure`, as `Button`
  does), so the folding is precise rather than estimated.
- Greedy packing: if every action fits in the budget (width − leading − title −
  margins), everything goes inline, with **no** `⋯` button; otherwise the `⋯` is
  reserved and as many actions as possible are kept inline, with the rest going
  into the menu.
- The overflow menu reuses [`Menu`] (controlled: opened and closed by the app,
  deferred overlay, dismissal on an outside click) — so it lives in the retained
  tree, not in a `LayoutBuilder` (whose content has no overlay).
- The width is **passed in** by the app (it is the size the app already has, not
  a platform indicator): consistent with the "single codebase" principle.

## Validation

- `frus-widgets`: `wide_bar_shows_all_actions_inline` (3/3 inline),
  `narrow_bar_collapses_into_overflow` (folding + `⋯`). 120 tests green.
- **On the device** (Android, ~232 logical px wide): a clean header
  `[☰] My Tasks · [Pause] [⋯]`, with no overlap left; the `⋯` menu drops down
  Light / A+ / A− / Log → / Settings → / Quick actions / Save / Clear completed.
- Demo: no `SizeClass` branch in the header; the old `Wrap` row + `Menu` +
  spinner disappears in favour of a single `AppBar::new(...).build()`.

## Limits (→ 52b)

- The AppBar is **placed in the scrolling body**; it ought to be **pinned** at the
  top (fixed chrome), just like the bottom navigation bar at the bottom. That is
  a `Scaffold`'s job (appBar / scrollable body / bottomNavigationBar) —
  milestone 52b.
- The `leading` reserves a fixed width (56 px); a wide leading would be
  underestimated in the budget.
- Touch targets are still under 48 dp (a dedicated milestone).
