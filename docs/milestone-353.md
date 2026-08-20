# Milestone 353 — A grid's tile has a shape

Milestone 351 left two questions open: whether a **scroll** hands its content a box, and
whether `Grid` and `Table` hand out cells the way the reference does. Both were checked
here. Two of the three were already right, and the third — the grid — was missing the
thing that makes a grid a grid.

## What was already right

**A scroll's content.** `compute_scroll` fills the constrained axis: when exactly one
axis is free — true single-axis scrolling — the content's unset dimension takes the
viewport's, so a vertical scroll's child is as wide as the viewport rather than as wide
as its words. When *both* axes are free, which is a two-dimensional scroll, it leaves both
alone, which is also right and is why the rule is written as "the axis that is constrained
while the other is free" rather than "the cross axis". The note in milestone 351 assumed
otherwise without reading it.

**A table's cells.** `cell_style` gives a cell either the column's fixed width or
`flex_grow: 1` in a stretch-aligned row: tight across, free down, with a `ROW_H` floor.
That is the reference's `BoxConstraints.tightFor(width: columnWidth)` — tight width, the
height left to the content — including the part where the tallest cell sets the row.

## What was missing

The reference's grid delegate computes, from the track it has just sized, a **main-axis
extent for every tile**:

```dart
final double childCrossAxisExtent = usableCrossAxisExtent / crossAxisCount;
final double childMainAxisExtent = mainAxisExtent ?? childCrossAxisExtent / childAspectRatio;
```

and hands each child a box that is tight on both axes. `childAspectRatio` **defaults to
1.0**: a grid's tiles are square unless the application says otherwise.

Ours had none of it. Rows followed their content, so a board of tiles came out as ragged
bands — a photo grid with a 4:3 photo beside a 1:1 one has two different row heights, and
a grid of cards is as tall as whichever card has the longest label. There was also one
`gap` for both axes, where the reference has `mainAxisSpacing` and `crossAxisSpacing`
separately, because a tile grid usually wants them different.

## What it is now

```rust
Grid::new(3).gap(8.0).aspect(1.0)          // square tiles, at any width
Grid::new(3).column_gap(8.0).row_gap(16.0) // spaced apart separately
Grid::new(3).tile_height(120.0)            // an exact extent, the reference's mainAxisExtent
```

- **`aspect(ratio)`** — `width / height`, taffy's convention and the reference's. The
  height follows from the column's width, which follows from the grid's, so the same board
  is square on a phone and square on a desktop with wider tiles.
- **`tile_height(px)`** — a fixed row extent, whatever the width. It wins over `aspect`,
  as the exact number wins over the ratio in the reference.
- **`row_gap` / `column_gap`** — either spacing on its own, `gap` still setting both.

Two of the three are container styles (`Style::grid_row_height` → taffy's
`grid_auto_rows`, and the two gaps → taffy's two-axis `gap`). The ratio is not: it belongs
to the **children**, and a child cannot be asked for it, because a tile does not know how
wide its column came out. It is imposed during the walk, through
`Layout::set_tile_shape`, next to the fill request and the shrink grant — the third thing
now resolved there rather than written into a style, for the same reason all three are.

A cell that has chosen a size of its own keeps it. The grid's shape is a default for the
cells that did not say, not an override of the ones that did.

## The default stays content-height

The reference's ratio defaults to 1.0. Ours defaults to *no ratio*, and that is a
deliberate difference rather than an omission.

`Grid` is not only the reference's `GridView`. It is also the framework's plain CSS grid,
and four widgets are built out of it — the colour picker's swatch board, the date picker's
weekday header and month, the time picker's hour and minute pads. Making squares the
default would reshape all four silently, to no one's benefit; the reference does not have
that problem because its `GridView` is a scrolling tile view and nothing else is built on
it. A grid of tiles asks for tiles with one word, and the doc comment says which word.

## A trap found while doing it

`ColorPicker` *is* its grid: it forwards `style` and `children` and paints nothing itself.
A hook resolved during the walk, rather than read off the style, is lost by that kind of
forwarding — the same class of bug as the transparent wrappers that had to learn to
forward `stack`/`continuous`. It forwards `tile_shape` now. Nothing depends on it yet
(swatches are a fixed 28 px), which is exactly when it is cheap to fix.

## Left

- **A grid whose column count comes from a tile width.** The reference's other delegate
  takes a `maxCrossAxisExtent` and derives the count: `ceil(width / (max + spacing))`,
  at least one, then equal columns of what is left. It is the delegate a responsive photo
  grid actually uses. It cannot be expressed as a style, because the count depends on the
  grid's computed width, and it is not CSS `auto-fill` either — auto-fill sizes tracks
  *at least* the given extent and would give one column fewer. It needs the width before
  the layout, which is what `LayoutBuilder` is for, and `LayoutBuilder` is a layout leaf
  whose height does not follow its content. That is the real work.
- **`Grid` places cells one per track.** The reference has no column/row *spans* on
  `GridView` either (that is `SliverGridDelegate`'s business, or `Table`'s), so this is
  not a deviation — but taffy's grid can do it and applications will want it.
