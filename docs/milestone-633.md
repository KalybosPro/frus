# Milestone 633 — The container, as the reference's

## Objective

The next widget in the review against the reference (`docs/reference-review.md`). The
reference's `Container` (`widgets/container.dart:246`) is defined less by what it paints than
by **how big it is**, and on that frus's differed:

| | reference (`container.dart:387`) | frus before |
|---|---|---|
| no child, no size | the room its parent allows (`LimitedBox(0, ConstrainedBox.expand)`): all of it alone in a box or across a row, nothing along a row's main axis | nothing |
| an `alignment` | fills its parent and places its child inside (`Align`) | its child's size, the alignment doing nothing |
| `constraints` | minimum and maximum width and height, which a given width or height is held within | none |
| `transform`, `transformAlignment` | painted, about a point of the box | none |

frus's container also does what the reference spreads over other widgets: clicks and
hover colours (the reference's `InkWell` and `GestureDetector`), implicit animations
(`AnimatedContainer`), a flex factor, an opacity and a repaint boundary. Those stay. They
are additions, and an application that uses them loses nothing.

## What was done

- **The two sizing rules.** A container with no child, or with an `alignment`, asks for the
  room on every axis it was given no size on (`fill_axes`). Alone in a box or across a row
  or a column, the room is granted. Along a line shared with other children it is not, so
  an empty container is nothing there, as the reference's `LimitedBox(0)` makes it in
  unbounded room.
- **Constraints**: `min_width`, `max_width`, `min_height`, `max_height`. A width or height
  given with them is held within them, as the reference tightens them.
- **Transform**: `translated`, `scaled`, `rotated`, about `transform_alignment` (the box's
  centre unless told). They are painted and do not move the layout, as in the reference.

## Found on the way

- **A fill request now stops at an aspect ratio and at a grid.** frus answers "fill" by
  passing the request up the tree until a box with a size of its own stops it. The reference
  instead hands tight constraints down: an `AspectRatio`, and a grid's tiles, decide their
  child's size. An empty container inside either one used to reach past it and stretch the
  whole page. A box with an aspect ratio, and a grid, now end the request.
- **`Aligned` and `Center` are built on a container**, and decide for themselves how much
  room to take (their width and height factors). Their inner container sizes itself by its
  content (a crate-private `hugging`), so that the factors keep working.
- **A known difference.** frus grants an "only a default" size the room of a row or a column
  it is **alone** in, where the reference gives a row's child no room along it. An empty
  container alone in a row therefore takes the row's width, where the reference's would be
  nothing. With any neighbour the two agree. This is frus's rule for every widget with a
  default size (milestone 590), and changing it is a decision of its own.

## What it changes on screen

Two goldens, the collapsing header open and collapsed: the header's title is now centred,
and the rule under the collapsed bar is now drawn. It had been an empty container, so it
was zero wide and drew nothing. The header is a
column holding the title and a one-pixel rule made of an empty container. The rule now
takes the column's width, as the reference's does, the column is as wide as the bar, and a
column centres its children across, as the reference's does by default. The reference
would draw this tree the same way.

## Tests

- `an_empty_container_takes_the_room_allowed`: the whole of a centre; nothing along a
  shared row, all of its height across.
- `an_aligning_container_fills_its_parent`: fills, with its child at the bottom right;
  without an alignment, its child's size.
- `constraints_hold_the_box`: a floor, a ceiling holding a given width, and a ceiling on an
  empty container.
- `a_transform_is_about_its_alignment`.
- Three existing tests had used an empty container as a box that hugs or as a spacer with no
  width. They now say what they mean, with content or with a width.

Mutation testing: six mutants. Five were killed: the alignment rule dropped, a ceiling
dropped, the transform pivot ignored, a grid that passes its cells' requests on, and an
aspect ratio that does. The sixth, the "only a default" declaration (`soft_extent`) turned
off, survived. It turned out to change nothing for a container, whose default size is no
size at all, so it was removed.
