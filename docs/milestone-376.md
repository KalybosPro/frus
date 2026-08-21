# Milestone 376 — A grid that builds what you can see

`GridView` had two forms and both built every cell, every frame. Fine for a dozen colour
swatches. Wrong for two thousand photographs — which is the same argument milestone 375
made about lists one step earlier, and it does not stop being true because the cells are
arranged in rows.

`GridView::builder(columns, count, build)` is the third form, and only the visible rows are
built, laid out and painted.

## A windowed grid is a list of rows

That is not an analogy, it is the implementation. The grid reports a `VirtualList` whose
item is a row of cells, and from there nothing new windows, measures or scrolls: it is the
list's machinery, which is why a windowed grid scrolls, flings, reverses and shows a
scrollbar without a line of any of that being written here.

The test says so directly — `it_scrolls_like_the_list_it_is_made_of` asserts on
`scrollable_maxes` and on where a tile lands after a scroll, and neither the grid nor this
milestone contains scrolling code.

## The hook had to learn the box

The row height comes from the tile shape and the width the grid was given; the **window**
comes from the row height. So a grid cannot say how many rows fit until it knows how wide
its columns came out — and `virtual_list(&self)` was not told.

It takes the viewport now. Five small edits: the trait's default, the `Box<dyn Widget>`
forward, the transparent-wrapper macro, `Responsive`, and `ListView` itself, which ignores
it. Two call sites ask only *whether* a widget windows its children rather than *what* it
would build, and they pass `Size::ZERO` with a comment saying so — a size that will not be
read is the honest argument, and inventing a second `virtualises()` predicate to avoid
writing it would be a second thing to keep in step.

## What the factory knows, and what it must not

The row factory is composed **once**, in a `OnceCell`, and captures the column count, the
spacing and the cell factory — none of which depend on the width.

The row **height** does depend on the width, and it is deliberately not in there. It is the
item extent, computed fresh on every call, and the gap below a row is the row's own bottom
padding. Keeping those apart is what makes the `OnceCell` correct: a factory that knew the
height would be wrong the moment the window was resized, and a `OnceCell` cannot be
rebuilt.

## Three decisions in the arithmetic

**Square by default.** The window is found by division and division needs a number, so
there has to be a default when neither `aspect` nor `tile_height` is given. A square is
what a grid of photographs means when it says nothing.

**A short last row keeps its columns.** Four tiles across three columns is a full row and
one lonely tile, and that tile is a third of the width — because "three columns" did not
stop meaning three on the last row. The missing places are filled with empties that take
their share and draw nothing.

**The gap goes below the row, not around it.** The list hands its item the whole extent, so
a row that stretched would eat the spacing; the row is `tile_h` tall with the gap as bottom
padding underneath. Getting this wrong is invisible in a single row and wrong in every
grid, which is why there is an assertion for it.

## `Msg: Clone` on the grid's `Widget` impl

`GridView`'s implementation asked only for `Msg: 'static`, and the row it now builds is a
`Flex`, which asks for `Clone`. So the bound moves up — to `GridView` and, following it, to
`ColorPicker`, which holds one.

It breaks nothing. `build_ui` has always required `Msg: Clone`, so a message type without
it could never have reached a screen; the looser bound described a widget that compiled and
could not be shown.

## Left

The **plain** forms still build everything, and that is right: `GridView::new` takes cells
the application already made, and there is nothing to defer. `GridView::extent` derives its
column count from the width and could be windowed the same way — it wants
`columns_across` inside the same `virtual_list`, which is a small step and a separate one.

A windowed grid does not scroll **horizontally** yet. `ListView` gained that axis one
milestone ago and the grid hard-codes `Axis::Vertical`; the reference's
`scrollDirection` on a grid transposes rows and columns wholesale, which is more than
passing the axis through.
