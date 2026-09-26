# Milestone 579 — `Center` and `Aligned`

Centring something was a `Container` with an alignment and nothing else, and that was not
enough: a `Container` with no size is only as big as its content along a flex line. Centring
something in a page body, an empty state or a spinner meant knowing to add a size or a flex, and
getting it wrong put the child at the top left with nothing to say why. The most common layout
request in an interface had no widget of its own.

## What it does

- **`Center::new(child)`** centres the child in the room it is given.
- **`Aligned::new(alignment, child)`** places the child at any anchor: an `Alignment`, a fraction
  between two, or an `AlignmentDirectional` that mirrors in a right-to-left script.
- `center(child)` is the DSL shorthand, beside `expanded` and `spacer`.
- **How much room they take**, the reference's rule for the box that aligns a child:
  - all of a bounded axis — a page body under its bar, a sized box, a card, a grid cell;
  - the child's length along a row's or a column's own direction **when the line holds other
    children**, since a line shared with others hands its children no length (a centre in a
    column is centred across the column only, just below what comes before it;
    `Expanded::new(Center::new(…))` centres it in the rest);
  - the child's length along an axis with no bound at all, down a scroll for one.

## How

- Both are a `Container` with an alignment, which already places a child at a fraction of the
  free space and resolves a directional anchor. What they add is the size: they ask to fill both
  axes (`Widget::fill_axes`). The layout walk already turns that request into the reference's
  answer: fill where the parent hands down its own constraints, and fall back to the child's size
  along a line shared with siblings (milestone 405).
- **The name.** `frus_widgets::Align` has been the cross-axis alignment of a row or a column
  since the first layout milestones, and renaming it would break every application that aligns
  a row. So the widget is named for what it does to its child, as `Positioned` and `Expanded`
  are. The reference calls it `Align`.

## Checked against the reference

The reference's box that positions a child (`RenderPositionedBox`) takes the maximum of its
constraints on an axis unless that axis is unbounded or a size factor is set; then it takes the
child's size, times the factor. Its children are laid out loosely. That is the behaviour above,
without the factors.

## Verification

- Frame tests (the square's rectangle as painted):
  - alone on a page, centred on both axes;
  - in a sized box, centred in the box;
  - in a column after a title: centred across and just below the title, and with `Expanded`,
    centred in what the title leaves;
  - in a row after a label: centred down the row, just after the label;
  - as a page's body under a 56-px bar: centred below the bar;
  - in a scroll: at the top, centred across;
  - `Aligned` at a corner, at a fraction, and at a directional start.
- Mutations, each failing a test:
  - no fill request;
  - `Center` not passing its request on;
  - no anchor;
  - `Center` anchored top-left;
  - filling the width only.
- Not run on the phone: no device was attached. Nothing here is platform code; the layout and the
  paint are what the frame tests read.

## Left

- The reference's `widthFactor` / `heightFactor`, which size the box to a multiple of its child.
  They need the child measured before the box is sized, which is issue #52.
- The demo still centres its task page's words with a `Container` of its own (milestone 574):
  that box is given the full width on purpose, and a `Center` would not change that.
