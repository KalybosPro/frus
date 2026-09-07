# Milestone 457 — Properties a list tile and a divider did not have

A batch rather than a theme: three properties on `ListTile`, one on `Divider`, all of them
in the reference and none of them here. Batched deliberately — they are independent, they
touch two files, and one verification run covers all four.

## A selected tile had no surface

The reference's `ListTile` has `selectedTileColor`. This had `selectedColor`, which is the
colour of the **words**, and nothing else. So being *the one you are on* changed the colour
of the text and nothing about the row.

That is the weakest possible way to say it. It is the difference between a highlighted row
in a navigation list and a row that merely reads differently — and in a list of ten rows,
the second one is close to invisible.

It resolves the way the reference's does: while the tile is selected,
`selected_tile_color` outranks `tile_color`; unselected, it says nothing at all. A tile
that names neither still paints nothing, which is what a tile on a page ought to do.

`icon_color` and `text_color` came with it: the reference has both, over what the selection
would otherwise give them.

## A tile had no shape, and the ink knew it

`ListTile::shape`, with the surface **and the ink** taking it. A rounded tile whose splash
still had square corners would be worse than one that was never rounded — the ink is the
thing the eye follows, and it would leave the tile's outline for the length of every tap.

`InkStyle::radius` already existed for exactly this; nothing had ever set it from a tile.

## A rule could not round its ends

`Divider::radius` (`divider.dart:68`). A hairline wants square ends and keeps them —
nothing named a radius, nothing draws one, and the fast `fill_rect` path is still what a
plain divider takes. A **thick** rule is a different object: it reads as a bar, and a bar
with square ends is about the only thing left in an interface that still has them.

## The tests

- `a_selected_tile_has_a_surface_of_its_own` — all four combinations of the two colours,
  selected and not.
- `a_tile_takes_a_shape_and_the_ink_takes_it_too`.
- `a_rule_can_round_its_ends` — including the hairline that stays square.

**The goldens did not move**: every one of these is opt-in, and nothing in the pictures
opts in.
