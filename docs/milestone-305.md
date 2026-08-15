# Milestone 305 — A scaffold that does not change its mind

Reported from the device: *"in the demo, when I turn the phone to landscape, the
side-by-side comes. That is not what the reference does by default."*

Both halves of that are right.

## What it was doing

`Scaffold::build` measured its own width and decided:

```rust
let compact = SizeClass::from_width(width) == SizeClass::Compact;
...
if compact { BottomBar::new(..) } else { NavRail::new(..) }
```

No parameter, no way to ask it not to. A phone in portrait is `Compact`; turned to
landscape it is `Expanded`, so the navigation left the bottom of the screen and
reappeared as a column on the left edge — because the user rotated their hand. Nothing
in the application had asked for that, and nothing in the application could prevent it.

The word for that in the module's own documentation was "adaptive", which is how it
survived: it reads as a feature.

## What the reference does

Nothing. Its screen shell has one navigation slot — `bottomNavigationBar` — laid out at
the bottom unconditionally; `scaffold.dart` contains no width breakpoint and never
mentions a rail. The rail is a separate widget, placed by whoever wants one:

```
$ ref grep "NavigationRail|MediaQuery.of(context).size.width" --glob "**/material/scaffold.dart"
no match (1 files searched)
```

Which is the right shape, and for a reason worth stating: whether navigation should
move when a window grows is a **design** decision. The framework does not know whether
this application's three destinations belong at the bottom on a tablet or down the
side. The application does.

## What it does now

`Scaffold`'s navigation is a **bottom bar**, at every width. That is the default and it
does not depend on anything.

The rail is not lost — it is asked for, and once asked for it is fixed:

```rust
Scaffold::new(width, height)
    .nav(app.section, Msg::SetSection)
    .nav_placement(NavPlacement::Rail)   // and it stays a rail, at any width
```

`NavPlacement` has two variants, `Bottom` and `Rail`, and deliberately **no**
`Adaptive`. An adaptive third variant was written first and then removed: it would have
put the size-class branch back inside `Scaffold`, one opt-in away, and left the type
name promising something the widget could not keep — that a scaffold's layout is what
you asked for.

Navigation that follows the size class lives in `NavScaffold`, a separate shell whose
entire purpose is that and which is named for it. It takes the `SizeClass` as an
argument, so an application choosing it has already said, in writing, that it wants its
navigation to move.

## The demo

Left on the default, and the comment at the call site says so rather than the code
saying it, because that is how an application would write it. Turning the phone now
changes nothing about where the navigation is.

The Stats screen still splits into two panes when there is room. That one is
`TwoPane`, constructed by the demo with a size class the demo chose — the application
asking, which is exactly the arrangement this milestone is about. It is one line to
change if the demo should stop.

## Verification

- Three new tests in `scaffold.rs`, and the first is the milestone:
  - `a_wide_window_keeps_the_navigation_at_the_bottom` — 900 × 420, well past the
    threshold, and the destinations are still painted in the bottom band. This is the
    test that fails against the old code.
  - `a_rail_is_drawn_when_it_is_asked_for` — same tree, same width, `NavPlacement::Rail`:
    the destinations move to the leading edge and stack from the top.
  - `a_rail_survives_a_narrow_window` — a placement is a decision, not a hint the
    scaffold may overrule.
- 977 workspace tests, 128 pixel tests with **no golden changed** — no golden rendered
  a scaffold wide enough to have grown a rail, which is its own small comment on the
  coverage.
- A fourth test, in the **demo**, because a widget test can pass while the application
  still looks wrong: `rotating_the_phone_leaves_the_navigation_at_the_bottom` builds the
  demo's real view at the reporter's own logical size — 424 × 918, then 918 × 424
  through `Application::on_resize` — and asserts the destinations are painted in the
  bottom band both times.

  It was checked against the defect rather than merely written: with the old
  width-sniffing put back for one run, it fails with
  `landscape: destinations at y = 168 of 424 — the navigation moved`, which is the rail
  anchored to the top. A regression test that has never been seen to fail is a guess.
- The README's pictures are re-rendered: the 900 px stills used to show a rail on the
  left and now show the bottom bar.

**Owed: the check on the device.** The phone came off the wire between the build and
the run. The test above is the reporter's exact geometry through the application's own
`view`, which is the next best thing, but it is not the phone.

## Left

`NavScaffold` is thin next to `Scaffold` — destinations and a body, no app bar, drawer,
FAB or sheets. An application that wants adaptive navigation *and* the rest currently
has to build the outer shell itself. Either it grows the slots, or `Scaffold` learns to
take a pre-built navigation widget in a slot, the way the reference's
`bottomNavigationBar` takes any widget.
