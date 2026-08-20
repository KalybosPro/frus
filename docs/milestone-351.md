# Milestone 351 — A list hands its children a box

A list's child is not asked how wide it would like to be. It is **told**: the reference
gives a list's children a tight cross-axis extent, and a fixed-extent list gives them a
tight main-axis one too. Both numbers are the list's, and neither is a negotiation.

Ours asked. The item was laid out under `Constraints::definite` — constrained, so it
could not exceed the viewport, but free to be smaller — and an item that set no width
hugged whatever was in it. A list of coloured rows painted a **column of chips down the
left** instead of rows across the list.

Found on a device at the end of milestone 349, on the journal screen: five thousand rows,
each a container with a background and a border around a short label, each one drawn 79 px
wide in a 363 px list.

`Constraints::filled` already existed — a `PageView`'s page is handed its box the same way
— so the fix is that word.

## Why no test caught it either

The same reason as the last one, and it is becoming a pattern worth naming: the fixture
was too plain to show it. `stack_grid_list` builds its items out of bare text, and a line
of text hugged by its box and a line of text in a box across the list **draw the same
pixels** — it is left-aligned either way. The defect only appears once an item has a
background, a border or an alignment, which is to say once it looks like a real row.

`an_item_is_handed_the_list_s_width` gives an item a fill and reads the rectangle back.

## Confirmed where it was found

`XMJNW19B23011768`, release build, the journal screen: the rows run the width of the list,
alternating backgrounds and borders across it instead of down the left. Which is what a
journal of five thousand rows was always supposed to look like.

## Left

- **A scroll's content is still only constrained on its cross axis**, not filled. The
  reference gives a vertical scroll's child a tight width. Nothing in the demo depends on
  the difference today — the screens that scroll set their own widths — but it is the same
  deviation one widget along, and it wants its own look because a two-dimensional scroll
  leaves *both* axes free and must keep doing so.
- **`Grid` and `Table` hand out cells** the same way a list hands out rows, and have not
  been checked against the reference's constraints.
