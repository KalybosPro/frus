# Milestone 719 — Scrolling to a row by its index

## Objective

A list could be scrolled from code to an offset, its start or its end. What an application
usually wants is a **row**: the message a notification pointed at, the selected item of a long
list, the search result the person picked. With rows of one height the offset is arithmetic the
application can do; with `ListView::builder`'s rows of any height (milestone 716) it cannot, as
nobody knows where row 240 is until the rows before it have been measured. Milestone 717 left
it for this one.

## What was done

- **`ScrollTo::index(n)`**, a new `ScrollTarget::Item { index, align }`, and
  `ScrollTo::aligned(f)` for where in the window the row comes to rest (`0.0` its leading edge,
  `0.5` its middle, `1.0` its far edge). Along the axis the list runs, whichever it is.
- **The frame records each list's shape** — its leading padding, its item extent (the guess,
  for rows of any height), its count, its direction, its window and its largest offset — in
  `Runtime::list_shapes`. `Runtime::scroll_to` turns an item into an offset from it, through
  `ListShape::offset_of`, which uses the measured extents for rows of any height.
- **The row is followed until it is there.** In a list of rows of any height the first answer
  is a guess: the rows on the way are measured as the list travels, and their mean — the guess
  for the rest — moves with them. So a request for an item is kept, and placed again at the top
  of each frame from what the last one measured: where a glide is heading while it glides, the
  offset itself once it rests, until the row is where it was asked for. A finger taking the list
  lets the request go.
- **The test driver counts a glide and the widgets' own values as movement.** `Tester::settle`
  stopped while a list was still gliding to where it was sent.

## Tests

- `tester.rs`: in a 500-row list of one height and in one where every third row is three times
  as tall, `ScrollTo::index(120)` brings row 120 to the top of the window; `ScrollTo::index(60)
  .aligned(0.5)` rests a row in the middle.
- Without the following (the request placed once, from the guess), the list of any height
  stops 24 pixels short — checked by running the test against that.

## Not done

- **A keyed list that is the very root of a component** is not found by `host::scroll_to`: the
  key's lookup and the layout give it different identities in that one arrangement. A list in a
  page — under a column, a scaffold — is found, which is every case the guide shows.
