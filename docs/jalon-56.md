# Jalon 56 — Frame phases: conditional build (build → paint)

The second engine item of §1 in `docs/prior-art.md`: **splitting the frame into
independently invalidated passes**, each running only if its "dirty" bit is set.
Milestone 55 provided the "layout" half (the relayout cache); this one adds the
**build → paint** split at the shell level.

## The key observation

In frus's Elm model, `view` is a **pure function of
`(app state, theme, size)`**. It **never** reads the `Runtime` — hover, focus,
scroll offsets, carets and interaction animation progress all live in the shell,
outside the view. Therefore:

> A frame of interaction animation (a hover rising, a spring scroll, a blinking
> caret) **does not change `view`'s output** — it only needs to **repaint** the
> tree that is already built.

Until now, though, every frame rebuilt the whole tree (`app.view()` + mount/exit
detection) before painting.

## The `build_dirty` bit

`App` gains a `build_dirty` flag. The **build** phase (`app.view()` + mounts +
capturing exits) only runs if the state could have changed:

```
need_build = build_dirty || app_animating || (no retained tree)
```

- `build_dirty` is set at **exactly** the six points that mutate the app's state:
  `dispatch` (any `Msg`), `on_resize`, `on_insets`, and the three back-gesture
  hooks (`back_gesture` ×2, `back_gesture_end`); plus surface (re)creation.
- `app_animating` (the return of `app.tick`) covers the *app's own* animations
  (theme fade, screen transition, gesture settle) which do change the state the
  view reads each frame.

Otherwise the **retained tree** (`self.tree`, already kept for keyboard routing)
is reused as-is. The **paint** phase — advancing the `Runtime`'s animations, then
`build_ui` (whose layout goes through milestone 55's relayout cache) — does run
on every animated frame.

## Why this is correct (and safe)

The risk is asymmetric: **building when it was not necessary is harmless** (just
slightly slower, as before); **skipping when it was needed** would be a bug (a
frozen UI). Since the app's state is mutable **only** through
`update`/`tick`/`on_resize`/`on_insets`/`back_gesture*` — all of which mark
`build_dirty` (or are covered by `app_animating`) — and since `view`/`theme` take
`&self`, the retained tree can never go stale without a rebuild being scheduled.
Hover, scrolling, focus and the caret only change the `Runtime`, never the view.

## Result

- Hover, inertial scrolling, a blinking caret, appearance/disappearance fades, a
  spinner: **paint only**, without rebuilding or reallocating the widget tree.
- Combined with milestone 55, such a frame does **neither** `view()` **nor**
  taffy — only the paint walk. That is the brief's "a hover only touches paint"
  discipline.

## Validation

- The whole suite green, behaviour unchanged: `frus-widgets` 129, `frus-core` 37,
  `frus-demo` 15, `frus-shell` 7, layout 3, gpu 4, text 2.
- `cargo build --workspace` with no warnings; the demo ran without panicking and
  with no borrow conflict. (The render loop is not observable under WSLg-root —
  llvmpipe software rendering — so correctness rests on the purity argument above
  plus the tests.)

## Possible next steps (§1 / §12)

- A real **per-node "dirty" list system** (not just per frame): repainting only
  the touched subtrees (damage regions + GPU scissor, §12).
- A **persistent** taffy tree reconciled by identity (beyond milestone 55's
  result cache) for incremental relayout within a root.
