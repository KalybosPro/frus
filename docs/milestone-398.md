# Milestone 398 — The pages went past the edge of the thing holding them

A `Navigator` slides one screen out while another slides in. Both start or end **outside**
its box — that is what sliding is. Until this milestone nothing stopped them there.

The only thing bounding them was whatever clip they had inherited, which for a full-window
navigator is the window. So it looked fine, and it was wrong in two ways:

- A navigator that is **not** the whole window — one in a pane, a master-detail layout, a
  card — painted its pages straight over whatever sat beside it.
- Even a full-window one spent every transition frame drawing a screen nobody could see.

The reference clips by default: `Navigator`'s `clipBehavior` is `Clip.hardEdge`
(`navigator.dart:1601`).

## How it was found

Not by a test. By a comment I had to write in milestone 393, in the demo's safe-area check:

```rust
// A `Navigator` draws the screen it is leaving beside the viewport, at a
// negative x or past the right edge
```

That test failed on text painted at x = 452 in a 400-wide window, and the exclusion was
written to get past it. Writing down *why* a test has to ignore something is how the
something gets noticed.

## The change

`Widget::navigator_clips`, `true` by default, consulted only inside the navigator branch of
the walk — so it costs nothing for every other widget. The walk intersects the clip with the
navigator's own box before rendering either screen.

`Navigator::clip_behavior(false)` is the way out, for a transition genuinely meant to spill.
A decision, not a default.

## What the framework's own guard caught

The commit did not compile clean the first time — a test failed:

```
a transparent wrapper would answer these for itself: ["navigator_clips"]
```

`transparent::the_macro_forwards_every_hook_the_trait_declares` walks the `Widget` trait and
checks the `transparent!` macro forwards every hook. A `Keyed` or a `Responsive` wrapping a
navigator would otherwise have answered `navigator_clips` for **itself** — `true`, by luck,
but by luck. The guard exists because that class of bug has bitten this framework before,
and it earned its keep here.

## Still open on the navigator

`observers` (route-aware widgets and analytics — third-party code that wants to know about a
push it did not perform), `transitionDelegate`, `onGenerateInitialRoutes` and
`restorationScopeId`. The first three are about a navigator that **owns** its route stack;
ours is controlled, and the application holds the stack, so what they buy here is narrower
than it looks and needs a design rather than a translation.
