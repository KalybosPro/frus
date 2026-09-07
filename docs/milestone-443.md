# Milestone 443 — A short rail could not scroll

Four properties the reference gives a `NavigationRail` and this one did not have.

## `scrollable`

A rail with more destinations than height ran them off the bottom of the screen. There was
no way to say otherwise; the reference's answer is one line (`navigation_rail.dart:543`).

What scrolls is the **group** — the destinations, plus whichever of `leading` and
`trailing` was not pinned. The pinned slots stay put, which is what pinning them is for.

Saying it leaves `group_alignment` with nothing to do, and that is not a special case: the
viewport fills the rail, so there is no spare room left outside it to place a group in. A
rail scrolls because it has less room than it needs, which is exactly the case where an
alignment had nothing to work in either. The two spacers that place the group are dropped
rather than left to compete with the viewport for the same space.

## `main_axis_alignment`

The flexbox question — start, centre, end, or one of the three ways of spreading the spare
room — asked of the **group** rather than of the rail around it, so `SpaceEvenly` puts
equal room between every destination and at both ends.

The reference documents that this "overrides `groupAlignment`", and the mechanism is worth
copying rather than special-casing: it sets the group's main-axis size to `max`
(`navigation_rail.dart:501`), the group fills, and the `Align` around it has nothing left
to align. Here the group takes `flex(1.0)` and the spacers go, for the same reason and to
the same effect.

## `use_indicator`

Turning the indicator off is not just removing a shape. With nothing behind it the selected
glyph stands on the rail's **own surface**, so `on_secondary_container` — the indicator's
content colour — would be a colour for a ground that is not there.

The reference keeps the whole arrangement for this case: with no indicator, both the glyph
and the label take the **accent** (`navigation_rail.dart:1221`, `:1211`). With nothing
behind it, the destination has to say *this one* by itself. The state layer follows the
same ground: with no indicator, a selected destination's ground is the rail's surface like
everyone else's.

A **bar** has no such property. The reference gives `useIndicator` to the rail alone, a
bar's destinations sitting side by side with nothing else to tell them apart.

## `elevation`

Zero by default, as the reference's is (`navigation_rail.dart:1236`) — a rail is separated
from the page by a rule, not by a shadow.

A raised rail **drops that rule**. A shadow and a hairline are two ways of saying the same
thing, and a widget that draws both says it twice: the mash-up `Card` was taken apart for
(milestone 407's "one widget drawing a shadow **and** an outline, which is none of the
three").

`NavRailTheme` gained `use_indicator` and `elevation`, so both have the middle rung.

## The tests

- `a_short_rail_scrolls_its_destinations` — including that the group went *into* the
  viewport and that the spacers went away.
- `spreading_the_destinations_replaces_the_alignment` — the group's own `justify` and
  `flex_grow`, which is the mechanism rather than the consequence.
- `a_rail_without_an_indicator_says_so_in_the_accent` — no pill drawn, and both inks.
- `a_raised_rail_drops_its_rule` — the shadow appears and the one-pixel rect does not.

No golden moved: every default is the one the rail already had.

## Still open

`indicatorShape` (`:1148`) still wants a shape abstraction this framework does not have,
and a destination's own `padding` (`:1159`) still wants the item's paint to inset itself
from its own box. Both are changes to how a destination measures, not fields to thread.
