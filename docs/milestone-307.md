# Milestone 307 — A sweep for what does not match, and the first two it found

Instruction, after milestone 305 turned up a widget that decided something the reference
leaves to the application: *"we redo everything we had done that does not respect
Flutter's context and functionality."*

So this is not a feature. It is a sweep: read the reference for widgets already written,
and correct what does not match. This note records the method, what came back **clean**,
and the two that did not.

## The method, and why the negative results are written down

Every check is against the reference's own source, not against memory of it. That
matters more than it sounds: milestone 306 found `AppBar`'s documentation arguing
confidently, in this repository's own words, for behaviour the reference does not have.
A widget's doc comment is not evidence about the reference. Only the reference is.

Two shapes of deviation have now been seen, and both are worth grepping for again:

1. **A widget deciding for the application** what the reference leaves to it —
   `Scaffold` swapping its bottom bar for a rail past a width threshold (305).
2. **A widget refusing to decide** what the reference decides per platform — `AppBar`
   never centring its title (306).

To which this milestone adds a third, which is the more common one:

3. **A widget that is not wrong so much as unfinished** — one measurement where the
   reference has four, and no way for the caller to change any of them.

### Clean

- **Scroll physics.** `ScrollPhysics::platform_default` is bouncing on Apple's
  platforms and clamping elsewhere, resolved at compile time. That is the reference's
  rule exactly.
- **The refresh indicator.** `DRAG_EXTENT_FRACTION` 0.25 and `DRAG_SIZE_FACTOR_LIMIT`
  1.5 — the reference's `_kDragContainerExtentPercentage` and `_kDragSizeFactorLimit`,
  to the digit.
- **`Dismissible`.** Horizontal by default, as the reference is.
- **No second `Scaffold`.** Nothing else in the widget set measures its own width to
  change shape: the only remaining `SizeClass::from_width` calls are a `MediaQuery`
  reporting one, a helper the application calls itself, and the demo's own choices.

## `Drawer`

Two things, one of which is only a number and one of which is not.

The number: the panel was **280 px** where the reference is **304**.

The other: `DRAWER_WIDTH` was a `const` used directly in the panel's style, with no way
to change it. A drawer is a slot an application fills with its own navigation, and a
framework that fixes its width has decided how wide that navigation is allowed to be —
which is exactly what the standing rule about customisation is against. `Drawer::width`
now overrides it, and the constant is the default rather than the rule.

## `Divider`

`Divider` was a unit struct — no fields, no builders, nothing to set — and it drew a
line by filling its whole box, which was one pixel tall.

The reference's separator has **two** measurements that this collapsed into one:
`height`, the room it takes in the layout, and `thickness`, the line drawn inside that
room. The defaults are 16 and 1: a line with air on both sides. Filling the box means a
separator can only ever touch the rows it separates, and asking for a thicker line meant
asking for a thicker *bar*.

It has both now, plus `indent` and `end_indent` — which inset the **line** and not the
box, the way a list separates rows without cutting through their leading icons — and
`color`.

Two smaller corrections came with it. The line is drawn in the theme's **discreet**
outline (`outline_variant`), where it used to take the full-strength `outline`; the
reference uses the discreet one, and a separator that is as strong as a border around a
control is competing with it. And a thickness larger than its box is clamped rather than
overflowing into the rows above and below.

**Breaking, and visibly**: a default `Divider` is 16 px of layout now instead of 1. An
application that wants the old flush hairline writes `Divider::new().height(1.0)`, which
is one call and says what it means.

## A test that was inverted, and passing

Widening the drawer broke `rtl_flips_the_drawer_side`, and the interesting part is why.

The test built a `Scaffold` with an **end** drawer — the trailing edge, right under LTR
— in a window of 200 px, against a panel of 280. Its assertions read:

```rust
// LTR: the drawer is anchored to the **left**, the start side.
assert!(edge(&ltr, 2) && !edge(&ltr, 197), "LTR: drawer on the left");
```

That is the opposite of what an end drawer does, and the opposite of what the test's own
doc comment two lines above says: *"an edge drawer moves to the left under RTL"*. It
passed anyway. A panel anchored to the right edge of a window **narrower than itself**
starts at a negative `x`, so the strip of its content that survived on screen was the
one at the left edge — and a probe at `x = 2` found it there.

Widening the panel by 24 px pushed that strip off the screen entirely, and the test
failed for the first time. It had been green for as long as it had existed, asserting
the reverse of the behaviour.

It now uses a window wider than a panel, so there is no overflow to read backwards, and
it measures the **middle of the run** of drawer-coloured pixels rather than probing two
hopeful ones — a probe at a fixed `x` cannot tell "the drawer is here" from "a sliver of
the drawer is here". The golden was re-blessed at the new size after looking at it.

Two things were tried before this and thrown away rather than kept. Clamping the panel
with `max_width: Percent(1.0)` — which does nothing, because the overlay layer the panel
is laid out in has no definite width for a percentage to resolve against, and shipping
it with a confident comment would have been worse than not trying. And blaming the
first `[[bin]]` suspect, which cost three rebuilds in milestone 306 and is the same
mistake twice: guessing at a cause instead of measuring one.

## Verification

- Seven tests on the divider, and the first is the milestone: that the box's height and
  the line's thickness are two different numbers, and that the line lands in the middle
  of the box with air on both sides. Then the flush hairline, the indents insetting the
  line and not the box, a thickness clamped to its box, indents wider than the box
  drawing nothing at all, and the caller's colour winning.
- One on the drawer, asserting both halves: the default is the reference's 304, and
  `width(96.0)` gets 96.
- 998 workspace tests, `clippy` silent on every target, `rustdoc` clean under
  `--all-features`.
- **Six goldens moved, and each was looked at before it was accepted.** Four from the
  divider — `icon_set`, `stepper_and_timeline`, `carousel_and_two_pane` and
  `navigation_pickers` — each showing what the change was meant to show: a discreet line
  with air on either side of it, where there was a full-strength bar flush against the
  rows. Two from the drawer: `drawer_open`, whose panel is 24 px wider, and `rtl_drawer`,
  re-rendered at the new window size. A golden accepted without being looked at is a
  golden that records whatever the bug now does.

## Left

The sweep is not finished — it has covered the widgets there was reason to suspect, not
all 94. Known and not yet done:

- **A drawer wider than its window overflows** instead of shrinking. The reference's
  width is enforced against the parent's constraints, so a 304 px panel in a 200 px
  window becomes 200. Here it keeps its 304 and slides its own content off the edge —
  which is exactly what kept the inverted test above green. It wants the overlay layer to
  pass a definite width down, not a `max_width` on the panel.
- **`VerticalDivider`.** The reference has one; this has only the horizontal.
- **`Drawer` has no edge drag.** The reference opens a drawer from a swipe starting
  within 20 px of the edge (`_kEdgeDragWidth`), which is how most people open one on a
  phone. Here it only opens from a button.
- **The theme carries no per-widget defaults.** The reference resolves nearly everything
  as `widget ?? theme.widgetTheme.x ?? defaults.x`, and the middle term does not exist
  here: a caller can override one divider, but an application cannot say "every divider
  in this app is 1 px" once. That is a design question, not a patch, and it is the same
  gap `splashFactory` ran into in milestone 306.
