# Milestone 767 — An autocomplete takes the width it is offered; table labels never overlap

## Objective

Two items left open by milestones 758 and 759:

1. `Autocomplete` had a fixed 260 px default width, wherever it was put. The leading
   toolkit's autocomplete builds a text field that takes the width its parent offers, and
   hangs its options under the field **at the field's width**.
2. A `Table` with flexible columns placed where the width is unbounded (in a row beside
   other children, in a horizontal scroll) drew every column at zero width, its labels
   painted on top of each other. The same question was owed for `DataTable`, `GridView` and
   `Skeleton` in a row.

## What the leading toolkit does

- **Autocomplete.** The default field is an ordinary text field. Its decorator takes the
  maximum width on offer and stops with an error when that width is unbounded ("cannot have
  an unbounded width ... contained by a Row"). The options view is laid out in a box exactly
  as wide as the field (`fieldSize.width`), whatever gave the field that width.
- **Table.** A table's default column width is a flex share. A flex column has no intrinsic
  width (`minIntrinsicWidth` and `maxIntrinsicWidth` are both 0). With a bounded width the
  table takes the maximum and divides the free space among the flex columns; with an
  unbounded width its target is the minimum width, normally 0, so the flex columns come out
  at **zero**. Nothing overlaps there only because its cells lay their text out: each label
  wraps inside a zero-wide column, one character per line.
- **Intrinsic columns** (`IntrinsicColumnWidth`) are as wide as their widest cell. They are
  what a data table uses by default: under unbounded width a data table is as wide as its
  content.
- **A grid** is a vertical scroll view, and a vertical viewport given unbounded width stops
  with an error ("Vertical viewport was given unbounded width").
- **A placeholder bar** is an empty box of a given height. With no width and an unbounded
  width on offer, an empty box comes out zero wide.

## What was done

### Autocomplete

- **The field takes the room on offer** (`autocomplete.rs`). Without `width`, the field is
  built unsized, so it asks to fill like any text field (milestone 590): it is stretched
  across a column, grows alone in a box, and the autocomplete passes the request up. Its
  default of 260 px is kept only as the fallback where the room is unbounded — in a row
  beside other children, where the leading toolkit has no width to give and stops. That
  fallback goes through a crate-private `TextField::default_width`, which changes the soft
  default without marking the field sized.
- **The list is as wide as the field.** New `Widget::overlay_follows_anchor_width` (default
  `false`, forwarded by `Box`, the transparent wrappers and `Responsive`). When a widget
  answers `true` and its overlay hangs below it, the overlay pass lays the overlay out at the
  anchor's width — handed, not offered — and lets its height be its own. `Autocomplete`
  answers `true`; its suggestions (and the scroll viewport of a capped list) no longer carry
  a width of their own when none was given, and run the width of the list. A width given
  through `width(..)`, or through the field's own dressing (`field(|f| f.width(..))`), is the
  width the list follows.
- `width(..)` is unchanged: a width given is a width kept, for the field and the list.

### Table

- **Each flexible column has a floor: the widest label in it** (`table.rs`). The column's
  heading (with its icon, the room for a sort arrow when the table sorts, and the width of a
  heading action that states one) and every text row are measured once per pair of text
  styles, with the cell's padding on both sides, rounded up to whole pixels. Every text cell
  of the column carries that floor as its `min_width`. It is the column's floor rather than
  the cell's, so that every row's edges stay in the same place.
- **Where the width is unbounded**, a flexible column is now as wide as its widest label, and
  the table as wide as its columns: nothing overlaps, and the table reads as it would with
  intrinsic columns.
- **Where the width is bounded**, the columns still share it equally whenever every label fits
  its share, which is what milestone 758 settled and what every golden shows. A column whose
  label needs more than its share now keeps the label's width and the others divide the rest,
  where the label used to be drawn across the next column.
- **Every floored cell starts from a basis of zero.** With the default basis a heading that
  holds an action widget (`header_action`) started from its content and took a bigger share
  of its row than the cells under it took of theirs, so the heading's edges were not the
  column's: in the `table_header_action` golden, "Status" sat 44 px left of "Active". From
  zero, every cell of a column gets the same share and the same floor, and the heading's
  floor counts the action when the action states its width (a labelled button does).
- Fixed columns are not floored. `DataTable` builds a `Table`, so it gets the same floors.

This is a deliberate difference for the default flex column. The leading toolkit gives it
zero width under unbounded constraints and relies on wrapping inside the cell; a table cell
here paints its label on one line, so a zero-wide column can only overlap its neighbour. The
floor gives what the leading toolkit's own data table gives in that place (intrinsic columns)
instead of one character per line.

### GridView and Skeleton in a row — documented, not changed

- A `GridView` with no width in a row, beside other children and with no `flex`, has no width
  to take. The leading toolkit stops with an error there. Here it stays as wide as its tiles'
  content, as milestone 759 left it: readable, nothing invented.
- A `Skeleton` with no width in the same place is zero wide, which is also what the leading
  toolkit's empty box is there. Give it a `width` or a `flex`.

## How it was tested

- **Tests that fail before the change.** Run on a copy of `origin/master` with the new tests
  added (the sort-arrow test used the literal 12 for the new constant):
  - `autocomplete::tests::the_field_and_its_list_take_the_width_offered`: an autocomplete in
    a box capped at 340, in a start-aligned column. Field and suggestions must be 340 wide.
    Before: the field was 260.
  - `autocomplete::tests::a_capped_list_takes_the_fields_width` (`max_visible` list in a box
    capped at 320). Before: everything 260.
  - `autocomplete::tests::the_list_follows_a_width_given_to_the_field` (`field(|f|
    f.width(150))`). Before: field 150, list 260.
  - `table::tests::flexible_columns_keep_their_labels_apart_in_a_row` (a table beside a label
    in a row). Before: "Metric" at 51, "Value" at 53. It also checks that the second column
    lines up across rows and that the first column is exactly its widest label plus padding.
  - `table::tests::flexible_columns_keep_their_labels_apart_in_a_horizontal_scroll`. Before:
    "Name" at 10, "Role" at 12.
  - `table::tests::a_share_too_narrow_for_its_label_gives_way_to_the_label` (a 260 px table
    with a 164 px heading). Before: the next column started at 141, inside the label.
  - `table::tests::a_data_table_in_a_row_keeps_its_labels_apart`. Before: "Name" at 64,
    "Born" at 66.
- **Guards** that passed before and still pass: `a_width_given_is_kept`,
  `in_a_row_the_field_falls_back_to_its_default` (260 in a row beside a label),
  `a_fixed_column_keeps_its_width`, and milestone 758's
  `flexible_columns_share_the_offered_width` (equal shares when the labels fit).
- **Added after the copy of master was tested, so not run against it:**
  `a_sortable_heading_keeps_room_for_its_arrow` (now in a row beside a label) and
  `a_heading_with_an_action_stays_in_its_column`. The second is the misalignment the
  `table_header_action` golden showed on master.
- `cargo test -p frus-widgets --lib` and `cargo test -p frus-demo --lib` (the demo's About
  tab tests through `Application::view` included; its tags field sits in a row beside another
  box, so it keeps its 260 fallback), `cargo clippy --workspace --all-targets -D warnings`,
  `cargo fmt --all`.
- **Goldens** (`frus-test`, under WSL from an rsync'd copy, `CARGO_INCREMENTAL=0`): 236 of
  237 matched; `table_header_action` changed. Its heading row and its data rows now share
  their column edges ("Status" sits over "Active"), and the second column is wider than half
  because its floor holds the heading, its sort arrow room and the "Filter" button. That is
  this milestone's change; the golden was regenerated. No other image changed.
- **Example projects:** all eight (`widgets`, `authentication`, `counter`, `forms`,
  `hello_world`, `http`, `navigation`, `websocket`) build with `cargo test --no-run`, and the
  gallery builds for `wasm32-unknown-unknown`.
- **Renders before and after** (offscreen, `frus_test::Stage`, 420 x 900, against a copy of
  master and against this branch): the gallery's Autocomplete, Table and DataTable demos are
  pixel-identical (the Autocomplete demo gives a `width`, the tables give fixed widths or
  widget cells). Three extra scenes changed as intended: an unsized autocomplete in a box
  capped at 360 went from a 260 px field and list to 360 px for both; a three-column table in
  a row beside a label went from "Moons" and "Distance (AU)" two pixels apart to columns at
  their labels' widths; the same table in a horizontal scroll likewise.

## Not done

- **A table cell still paints its label on one line.** Where the width is bounded and the
  floors add up to more than the table has, the table now runs past its box instead of
  drawing labels over each other (whether the overflow band reports it was not checked); the leading toolkit
  would wrap the labels inside narrow columns. Wrapping a cell's label is a change to how
  cells lay out their text.
- **A virtualised table** (`virtual_rows`) builds its rows on demand, so its floors come
  from the headings only; a data label wider than its heading can still run into the next
  column there.
- **Widget cells** (`widget_row`, `widget_header`) keep the layout's own minimum, their
  content's; they are not part of the column floor, so a widget wider than the column's text
  can push its row's edges off the others'. A heading action that does not state a width is
  not counted either.
- **An autocomplete in a row beside other children** keeps a 260 px field: the leading
  toolkit has no width to give there and stops with an error; nothing here warns.
- **GridView / Skeleton in a row** were looked at and left as documented above.
- No phone check: the change is covered by the layout tests and the offscreen renders, and
  the demo's own autocomplete keeps its fallback width.
