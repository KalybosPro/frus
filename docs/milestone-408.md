# Milestone 408 — One guard, and a tripwire that caught its own tests

Milestone 407 fixed the reader's font size never reaching a real application. It fixed the
symptom. This fixes the shape that produced it.

## Eleven callers, one mistake

Every caller of `MediaQuery::scope` in the workspace writes the same line:

```rust
MediaQuery::new(size).scope(|| app.view(&theme))
```

The shell, the golden screenshot generator, the transform tests, the demo's own tests —
eleven of eleven scope the **build** and leave the measure and the paint outside. At a scale
of 1 nothing shows, which is why it survived four milestones. The API's shape invited it: a
closure looks like it bounds the work, and a frame does not fit in a closure.

## Two guards that had to agree

Milestone 407's fix left the shell holding two: `install_text_scale` for the frame, and
`MediaQuery::scope` around `view`. That is the same bug with an extra step. Whoever holds
one and forgets the other gets a layout measured at one size and painted at another, and
nothing says so.

`MediaQuery::install()` returns a `SurfaceGuard` that holds **both**, installed by one call
and released by one drop. They can no longer be held for different lengths of time.
`scope` stays, reimplemented on top of it — a closure is the right shape for a *subtree*,
where a widget changes what its children see.

## The tripwire

A debug assertion in `build_ui`: a text scale away from 1 with **no surface described**
means somebody installed half a surface, and the half they left out is about to be missing
for the layout that follows.

Debug only, because it is a wiring mistake rather than a state a running application reaches
— it should stop a test, not a user's frame.

## What it caught first

Its own subject's tests.

```
test text::reader_font_size::a_box_that_holds_text_grows_with_it ... FAILED
a text scale of 2 is installed with no surface described: something installed half a surface.
```

The three tests written in milestone 406 to prove that widgets follow the reader's font size
said `MediaQuery::of().with_text_scaler(2.0)` — and `of()` outside a scope is `UNSET`, a
surface of no size. They installed a scale with no description: **precisely the half-surface
the shell had been installing for four milestones**.

Those tests were not wrong about their result; text did grow. They were wrong about the
*setup*, and that gap — the harness arranging the condition differently from production — is
exactly what let milestone 403 ship broken. 1034 passed, 3 failed, and the 3 were the ones
about the reader's font size.

They now describe the surface they scale for: `MediaQuery::new(Size::new(400.0, 300.0))`,
the same size handed to `build_ui_inspected` on the next line.

## The tests

- `one_guard_installs_the_whole_surface` — both halves in, both halves out.
- `a_guard_holds_past_the_call_that_made_it` — a size resolved after the "build" and again
  at the "layout" gives the same answer. This is the assertion that has been missing since
  milestone 403.
- `surfaces_nest_and_unwind_in_order` — a subtree can still change what its children see.

## Left

- **The other ten callers still scope only `view`.** They are tests and the screenshot
  generator, all at a scale of 1, so none is wrong today — but each is the pattern, and the
  tripwire only fires when a scale is actually installed. Converting them is mechanical and
  is not done.
- **Still no test drives a whole frame.** The tripwire narrows the class rather than
  closing it: it catches half a surface, not a shell that installs none at all. The 🔴 on
  the roadmap stands.
