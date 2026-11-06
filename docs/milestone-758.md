# Milestone 758 — A table with flexible columns takes the width it is offered

## Objective

Seen on the phone (Huawei STK-L21, Android 10, release build of `frus-demo`): on the Settings
screen's About tab, the two-column table ("Metric" / "Value") drew its second column on top of
its first. Every row read as one smudge of two labels.

## Root cause

`Table` is built from `Flex` rows. A flexible column (the default, `column_widths` `<= 0`) is a
cell with `width: Auto` and `flex_grow: 1`, so it takes a share of whatever width its row has.
But nothing gave the row a width. A cell **paints** its label rather than laying it out, so to
the layout a row of flexible cells has no content at all, and the table's root column is sized
by its content unless its parent stretches it.

On the About tab the table sits in `ConstrainedBox::new(table).max_width(480)`. The box caps the
width but does not size its child, and the table, laid out along the box's main axis, came out
as wide as its content: two pixels, the gap between its two cells. Each cell was about zero
pixels wide, and each label was painted from its cell's left edge, two pixels apart.

Through `Application::view` at the phone's logical size, before the fix:

```
Metric       x=30.0 y=377.0
Value        x=32.0 y=377.0
Widgets      x=30.0 y=413.0
35           x=32.0 y=413.0
```

Any parent that does not stretch the table (a box that only caps it, a column aligned to the
start or the centre, a centred row) produced the same collapse. A table with a `width`, or with
every column fixed, was never affected, which is why the editable grid screen and the
`table_editable` golden looked right.

## What a table does elsewhere

In the leading toolkit, a table's default column width is a flex share of 1, and a flex column
has no intrinsic width of its own. When the width on offer is bounded, the table lays itself
out at the **maximum** width and divides the free space among its flex columns; it never sizes
itself to its content in that case. The table here now does the same.

## What was done

- **`Table` asks for the width on offer when it has a flexible column** (`table.rs`,
  `Widget::fill_axes`). When no total `width` is set and at least one column is flexible, the
  table answers `FillAxes::WIDTH`, the same request a row set to fill its line makes. The layout
  walk grants it the way it grants any other: it grows a lone child of a box, and it stretches
  the table across a column. A table with a `width` or with only fixed columns already knows its
  width and keeps answering as its root does.
- **`DataTable` forwards the request** (`datatable.rs`). It is a transparent wrapper: when its
  block is the bare table, the `DataTable` node *is* the table to the layout, so it has to pass
  the fill on along with the structure, as it already did for `style`, `children` and `stack`.
- **A doc link that did not resolve** (`restoration.rs`): `RestorableValue`'s doc linked to
  `BuildContext::use_restorable_serde`, a method that exists only with the `serde` feature, so
  `cargo doc` without the feature warned about an unresolved link. It now names the method in
  code and links `BuildContext`.

## How it was tested

- `table::tests::flexible_columns_share_the_offered_width`: a two-column table in a
  `ConstrainedBox` capped at 300, in a start-aligned column. The second heading and the second
  cell must start one share (151 px) after the first. **Fails before the fix** (`metric=10
  value=12`), passes after.
- `tests::the_about_table_keeps_its_columns_apart_on_a_phone` (`frus-demo`): the demo itself,
  through `Application::view` at a phone's logical size, Settings → About. Every second-column
  label must start more than 120 px after the first-column label of its row, about half the
  table along. Before the fix the gap was 2 px.
- The table and data table tests, the `frus-widgets` and `frus-demo` suites, clippy and the
  goldens (see the PR for the run).
- On the phone: a release APK of the demo, Settings → About, screenshot of the table.

## Not done

- **An unbounded width still collapses flexible columns.** A table with flexible columns placed
  where the width is unbounded (in a row next to other children, in a horizontal scroll) has no
  width to share, and its columns still come out at zero. The leading toolkit behaves the same
  there (a flex column has no intrinsic width), but it wraps the cell text rather than painting
  it over the next column. An intrinsic floor per column (the widest label in it) would make
  that case readable too; it is a change to how columns are measured and was left for its own
  step.
