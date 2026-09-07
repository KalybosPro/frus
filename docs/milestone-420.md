# Milestone 420 — A rule that stopped at the notch

Third of the four slots, and the same split as 417, 418 and 419: the shell says what there
is to consume, the widget in the slot consumes it.

```rust
row = row.child(inset_pad(n, insets.top, 0.0, bottom_clear, insets.left));
```

A rail padded from outside is a **shorter box floated inside the intrusions**. Everything the
rail paints is inside that box — including the one-pixel rule down its trailing edge, which
therefore began at the status bar and ended above the gesture bar. A rule that does not reach
the edge it is ruling off.

## What the reference does

```dart
// navigation_rail.dart:553
Material(
  elevation: elevation,
  color: backgroundColor,
  child: SafeArea(
    right: isRTLDirection,
    left: !isRTLDirection,
    child: Column(…),
  ),
)
```

`SafeArea`'s `top` and `bottom` default to `true`, so the set the rail takes is **the leading
side, the top and the bottom** — never the trailing one, which is where the body is. And the
`Material` is outside it, as in every other bar: the surface reaches the screen's edges, the
destinations stay clear of them.

The reference has no rail *slot* — the rail is composed by the application inside a `Row`, so
it simply reads the ambient description. frus has a slot, so the shell has to say what is in
it, which is what `MediaScope` is for.

## What changed

- `NavigationRail::style` grows by the leading intrusion and pads by the leading, top and
  bottom ones. Its `paint` already rules across the whole of `bounds`, so the rule now runs
  the full height of the screen without being touched.
- The `Scaffold` hands the rail slot a description: the top, the leading side and
  `bottom_clear`, with the **trailing** side removed.
- **The footer's leading side, beside a rail**, is now zero: the leading intrusion is inside
  the rail's own box and the footer sits in the column to its right. The `row_width`
  arithmetic did not have to change — subtracting the bare `RAIL_WIDTH` *and* `insets.left`
  is the same number as subtracting the rail's box, which is `RAIL_WIDTH + insets.left` now
  that the rail has taken it. Two lines that used to be right for the wrong reason are right
  for the right one.

## The test, and both of its halves

`a_rail_rules_the_full_height_and_holds_its_destinations_clear` puts a cutout down the
leading edge and bars top and bottom, then asks three things: the rule spans the screen, it
stands at `CUTOUT + RAIL_WIDTH - 1` — so the rail's box swallowed the cutout — and no
destination is drawn inside any of the three intrusions.

Each half was checked by breaking it:

- with the rail told but not consuming, the rule stands at `x = 75`: the rail never took the
  cutout, so the destinations sit in it;
- with the rail consuming but still padded from outside, the rule stands at `x = 123`
  (`24 + 24 + 76 - 1`) and spans `y = 40 … 390`: padded twice, and back to a rule that stops
  at the notch.

No golden moved (91 + 13 + 27).

## Still open

One slot left, and it is the one that is not a bar:

**the body**, which the reference does not inset at all (`scaffold.dart:3029`). It hands it a
description with the top removed when there is an app bar and the bottom removed when there
is a bar below, and a body that wants the notch avoided says `SafeArea` itself. That is a
change of contract for every screen ever written against frus, which is why it is last and
alone.
