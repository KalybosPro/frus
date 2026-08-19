# Milestone 342 — Row and Column

`Flex::row()` has laid children out in a line since the beginning. What it has never been
is a **`Row`**.

The difference is not the name. `Flex` is a flexbox container and answers flexbox's
questions the way flexbox answers them: it shrink-wraps its line, it stretches its
children across it, and its `start` is the left. The reference answers all three
differently — a row **fills the line it is given**, aligns its children on their
**centres**, and reads `start` as the start *of the reading direction*.

Every one of those is a bug generator:

- A row that shrink-wraps silently ignores `SpaceBetween`. There is no space to put
  between anything, so the setting does nothing and says nothing. Milestone 334 hit this
  in the demo and worked around it with `.flex(1.0)` on the row — the right spelling, the
  wrong default, and one every caller has to know.
- A row that stretches its children makes the icon beside a two-line label two lines tall.
- A `start` that means "left" puts the Arabic label on the wrong side.

So `Row` and `Column` are not aliases. They are the same machinery with the reference's
defaults, and with the two ordering knobs `Flex` never had.

## `MainAxisSize`, and why a widget cannot answer it

"Fill the line" sounds like a property of the row. It is not. It is a question about the
**parent**: fill means *grow* when the parent runs the same way, and *stretch* when it
runs across — and a widget has no idea what it was put inside.

So the row asks (`Widget::main_axis_fill`) and the layout walk answers, where both are in
view. Along an axis the parent runs too, the reference leaves the child's main axis
**unbounded** and `MainAxisSize.max` quietly degrades to `min`: there is no maximum to
take. The exception is a parent with a single child — a padding, an alignment, a decorated
box — which passes its own constraints straight down, and there the row does fill.

## The request has to travel

The first version of this stopped at the parent, and `Container > Column > Row` came out
20 pixels wide.

Follow it through: the column is as wide as its widest child; its widest child is the row;
the row stretches to the column. Nothing in that circle is bigger than the tile inside it.
The reference has no such circle because it works in constraints rather than in sizes: the
container hands the column a maximum width, the column hands it to the row, the row takes
it, and the column ends up as wide as the row it contains.

So the **request travels up** as the tree is built. Each container passes on the half of
its children's request that crosses its own axis — across it, the container's size *is*
the child's — and the whole request when it has a single child, because then it divides
nothing. It stops at any box that was given a size on that axis: that box has already
answered the question, and growing it past its own width would be answering a different
one.

That is `Fills`, and it is why `build_layout_scoped` now returns a pair.

## A root has no parent

Neither `grow` nor `stretch` means anything at the top of a tree. A **percentage** does:
it resolves against the room the layout is being computed in when that room is definite,
and falls back to hugging the content when it is not. So a row at the top of a frame fills
the window, a row inside a horizontally-free scrollable still shrink-wraps, and the same
row measured for its natural size is measured at its natural size. One rule, three right
answers, and no new plumbing through the constraint types.

## The two orderings

The reference gives a flex two directions, not one: `textDirection` orders the horizontal
axis and `verticalDirection` the vertical, and which of them is the *main* axis is what
tells `Row` and `Column` apart. For a row, the reading direction decides which end the
first child goes to and the vertical direction decides which edge `Align::Start` means;
for a column, the two swap jobs.

Here the reading direction is **ambient**: the frame is mirrored as a whole for a
right-to-left theme (milestone 268), which is what makes an application flip rather than a
container at a time. So a per-container reading direction can only mean one thing — this
run reads *against* the ambient one — and it maps to a reversed main axis, which is
exactly one reversal on top of the mirror. A row that spells out the direction it was
going to get anyway is not reversed at all, and there is a test that says so.

`FlexDirection` grew `RowReverse` and `ColumnReverse` for it, `Justify` grew `SpaceEvenly`
(the distribution `SpaceAround` is usually mistaken for), and `Style` grew `align_self`,
which flexbox always had and this layer had never needed.

## `Flex` stays

`Flex` is not deprecated and its defaults are not changing. It is the flexbox primitive —
wrapping, grids, `flex_basis`, `shrink` — and the sixty-odd widgets built on it want the
stretching, shrink-wrapping container they asked for. `Row` and `Column` are the
reference's defaults for application code, and nothing in the framework was rewritten to
use them.

## Left

- **A row inside a multi-child row does not fill**, matching the reference, because the
  reference gives it an unbounded main axis. It is the one case where "fill" reads as if
  it should do something and does not, so it is written down here.
- **`textBaseline`.** The reference takes an alphabetic-or-ideographic parameter on a
  baseline-aligned row; the text layer reports the alphabetic one, and milestone 341 gave
  the reasoning for not offering a name that resolves to the same number.
- **No overflow band.** A column with more children than fit reports an `Overflowing` and
  nothing is painted across the edge. Still on the roadmap, and now with a widget that
  makes it easy to provoke.
- **`clipBehavior`.** A run that overflows draws outside its box rather than being clipped
  to it.
