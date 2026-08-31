# Milestone 444 — A scrollbar on a phone

Frus drew a scrollbar on **every scrollable, on every platform, on both axes, always** — a
permanent 10-pixel bar with its own track, down the side of every scrolling page. On a
phone, where the reference draws nothing at all.

This one came from the person building an application with it, looking at their own screen.

## What the reference actually does

`MaterialScrollBehavior.buildScrollbar` (`app.dart:857`) is nine lines and answers two
questions:

```dart
switch (axisDirectionToAxis(details.direction)) {
  case Axis.horizontal: return child;                 // never, anywhere
  case Axis.vertical:
    switch (getPlatform(context)) {
      case linux: case macOS: case windows:  return Scrollbar(...);
      case android: case fuchsia: case iOS:  return child;   // nothing
    }
}
```

**Along the horizontal axis, never** — on any platform. **Down the vertical one, only
where the platform's own scroll views have a bar.** A finger already knows where it is on
the page; a permanent bar over the content is an affordance for a pointer that cannot feel
the edges.

And where it does draw one, it is smaller and quieter than this was:

| | reference | here, before |
|---|---|---|
| thickness | 8 (`scrollbar.dart:12`) | 10 |
| track | **transparent** unless asked (`:281`), 3–5 % when asked | `muted` at 18 %, always |
| thumb at rest | `on_surface` at 30 % dark, 10 % light (`:248`, `:242`) | `muted` at 55 % |
| shortest thumb | 48 (`:15`) | 28 |

The 55 % is worse than it reads, too: a translucent fill blends in **linear light** here,
so it painted heavier than the number said — the trap milestone 329 measured.

## `Scrollbars`

A two-valued policy beside `ScrollPhysics`, resolved the same way and for the same reason:
the reference asks this through `ScrollBehavior`, next to the physics, and **not** through
`ThemeData`. It is a platform behaviour, not an appearance.

```rust
pub enum Scrollbars { Never, Always }
```

`Scrollbars::platform_default()` is `Never` on Android and iOS and `Always` elsewhere,
resolved at compile time from the target — a build is for one platform, and a constant
keeps the choice out of the frame loop, exactly as `ScrollPhysics::platform_default` does.

Three rungs, as everywhere else here:

- `Application::scrollbars()` — the application's answer, defaulting to the platform's;
- `SingleChildScrollView::scrollbars(…)` — one area's own, over that;
- and the walk asks the widget first, then the runtime.

The runtime carries it (`Runtime::scrollbars`), set by the shell every frame beside
`still`, so an application that changes its mind while running is obeyed.

## What changed in the paint

The track is gone. The rectangle is still computed — it is what a drag on the bar is
measured against — but nothing is painted for it. The thumb takes the reference's resting
opacity for the surface's brightness, read off `surface.compute_luminance()` because this
crate's `ColorScheme` carries no brightness flag.

A bar that is not drawn is **not registered** either: `Ui::scrollbars` is empty, so there
is nothing to drag. That is the right answer rather than a side effect — a target a user
cannot see is a target they cannot aim at.

## The tests

- `a_touch_screen_gets_no_scrollbar` — and that the registry is empty with it.
- `a_bar_is_a_thumb_and_no_track` — one rectangle, at the reference's thickness, never
  shorter than a pointer can catch, at the reference's opacity.
- `a_sideways_strip_never_gets_one` — on a desktop, where the vertical one is drawn.
- `an_area_may_ask_for_its_own_answer` — both ways round.

## Still open

The reference's desktop bar **fades**: it appears while the area moves and fades out 600 ms
later over 300 ms (`scrollbar.dart:17`, `:18`), and thickens to 12 with a visible track
when a pointer is over it (`:305`). Both want the runtime to know when an area last moved,
which it does not record; recorded rather than guessed at.
