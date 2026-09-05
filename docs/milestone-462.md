# Milestone 462 — The place a tooltip goes, and no tooltip

`Placement::Tooltip` has been in the overlay system since it was built. It positions a
bubble above its anchor, **flips it below** when there is no room above, nudges it back
inside a window edge when it would overflow, and shows it **only while the anchor is
hovered** — a whole small behaviour, written, documented, tested through the portal, and
reachable from nothing:

```
$ rg 'Placement::Tooltip' --glob '!ui.rs' --glob '!portal.rs'
$
```

Not one widget in the crate used it. This is the widget that does.

## Why it matters more than it looks

`Tooltip` is not a decorative widget. It is the thing that makes an **icon button
legible**. A row of five icons with no labels is a guessing game for a sighted user and
silence for a screen reader; the reference's answer to both is the same widget, and this
framework's icon buttons have been shipping without it.

## What it is

```rust
Tooltip::new("Delete this row").child(IconButton::new(Icons::DELETE).on_press(Msg::Delete))
```

A wrapper. It shows nothing until the child is hovered, then floats a bubble over it.

Four decisions worth writing down:

**An empty message is not an error.** `Tooltip::new(hint)` where `hint` may be empty
becomes the child and no overlay at all, so a caller with an optional hint does not have
to branch around the wrapper. Same for `enabled(false)`.

**It forwards its child's layout.** A tooltip is not a box; it is whatever its child is,
with something floating over it. Milestone 425's rule — a transparent wrapper that
replaces its child's `Style` with a default turns a row into a column and nobody finds
out until the screen is wrong.

**The bubble is `inverse_surface`.** The reference builds the same thing by hand out of
white and grey, in a `switch` on the theme's brightness (`tooltip.dart:481`). The scheme
has had the role for exactly this since it was written, and using it means a tooltip
follows a palette the reference's does not.

**It wraps at 320 pixels.** The one number here the reference does not have — it
constrains a tooltip by the window instead — but a bubble as wide as a desktop window is
not a tooltip, it is a paragraph that appeared under the mouse. `max_width` moves it.

## What it deliberately does not do

`waitDuration`, `showDuration`, `exitDuration` and `triggerMode`. All four are about
**time**: how long a pointer must rest before the bubble appears, how long it stays, and
whether a long press brings it up on a touch screen. The overlay system shows the bubble
while the anchor is hovered and hides it when the pointer leaves, and that is the whole
of what it can express today — the gate is one equality against the hovered id
(`ui.rs:3782`), with nowhere to put a clock. Recorded rather than faked.

`verticalOffset` and `preferBelow` are missing for a narrower reason: the offset is a
constant in the placement arithmetic (six pixels, and the reference's is twenty-four
because it allows for a finger), and reaching it from a widget means carrying a number
through the overlay pipeline that only one placement would ever read. Also recorded.

And the semantics goes in as a **label**, not as the reference's separate `tooltip`
property. `SemanticsProperties::over` joins two labels on their own lines, so a reader
hears the hint and the control — but both platforms have a field for this specifically
(`AccessibilityNodeInfo.setTooltipText`, `AXHelp`) and this framework's
`SemanticsProperties` has no room for it. That is a change to the platform bridge, not
to a widget.

## `TooltipTheme`

Eight entries. A tooltip is the one widget an application is likely to put on *every*
icon it draws, so saying what one looks like on each of them is saying it a hundred
times.

## The tests

Seven, five of which fail when the widget is broken in the five ways it could be — the
bubble placed like a menu instead of on hover, an empty message still building one, the
wrapper eating its child's layout, the surface being an ordinary one, and the bubble
showing with nothing hovered.

`a_bubble_waits_for_the_pointer` is the one worth reading: it asserts an **empty frame**,
and then builds the same tree with a visible child to prove the emptiness was the
tooltip's absence and not the whole tree drawing nothing.

## The picture

**A new golden**, `tooltip_hovered`, in `motion.rs` rather than beside the settled
widgets — for the same reason a half-done swipe lives there. A tooltip's picture is not a
function of its arguments; it is a fact about the `Runtime`, and `Stage` is what can put
it there.

Getting the frame right took three tries, and both failures are in the test's comments:
with the caption below, the bubble covers it; with the button near the top, the overlay
flips the bubble **underneath** — which is the placement working correctly and not the
thing worth photographing.
