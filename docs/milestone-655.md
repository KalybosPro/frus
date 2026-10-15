# Milestone 655 — The primary scroll view is always scrollable

## Objective

Milestone 649 brought never and always scrollable physics, and left one rule of the
reference out on purpose until the owner chose (the owner chose to adopt it, 2026-10-14). A
scroll view given no physics is **always scrollable** when it is the page's primary one:

```text
physics ?? ((primary ?? false) || (primary == null && controller == null && vertical)
    ? AlwaysScrollableScrollPhysics() : null)
```

(`widgets/scroll_view.dart`, the `ScrollView` constructor.) A vertical list or grid given no
controller is primary, so a drag moves it, and bounces or glows at its edge, even when
everything is in sight. A page that fits still answers the finger as a page does.

The rule belongs to `ScrollView`, the base of `ListView`, `GridView` and `CustomScrollView`.
`SingleChildScrollView` has `primary` too, but no such default.

## What was done

- **`ListView::primary(bool)`**, and a list given no physics takes `ScrollPhysics::ALWAYS`:
  - when it is vertical and `primary` is unset;
  - or when `primary` is true, on either axis.

  Physics given are kept as given.
- **`GridView::primary(bool)`** and **`GridView::physics`**, which the grid did not have.
  The grid is vertical, so it is primary unless told otherwise.
- frus has no scroll controllers, so the reference's `controller == null` always holds.
- `ALWAYS` keeps the platform's feel and only settles the user's rule, as milestone 649 laid
  it over the scroll behaviour.

**Changed**: a vertical `ListView` or `GridView` with everything in sight now takes a
vertical drag: the drag bounces on the platforms that bounce, glows on those that glow, and
is not handed on to a scroll view around it. A list nested in a scrolling page, which the
reference also makes primary, should say `primary(false)` or `physics(ScrollPhysics::NEVER)`
to let the page scroll through it, as the reference's nested lists say `primary: false` or
`NeverScrollableScrollPhysics`.

## Alternatives weighed

- **Platform-dependent, as the reference's `PrimaryScrollController.shouldInherit` is.**
  That rule decides which *controller* a view inherits. The physics default in the
  constructor does not look at the platform, so neither does this.
- **Leaving it out** was milestone 649's choice, pending the owner's.

## Tests

- `frus-widgets`:
  - a short vertical list registers an area that is always scrollable, and takes a drag
    along it and not across it;
  - a short horizontal list takes none;
  - `primary(false)` turns the default off, and `primary(true)` turns it on across;
  - physics given are kept, clamping and never alike;
  - a grid is always scrollable unless told, keeps physics given, and takes `physics`.

Mutation testing: 7 mutants, all killed. They cover:
- the axis deciding the default;
- the physics the default takes;
- physics given winning;
- `primary` and `physics` each reaching the list and the grid;
- the grid's default.
