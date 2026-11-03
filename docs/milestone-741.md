# Milestone 741 — A selection over a list built on demand

## Objective

A leftover of issue #24, found while writing milestone 739. A `SelectionArea` around a
`ListView::builder` — or any list or grid that builds only the rows in view — took no part in a
selection: a press on a row began nothing, a drag could not reach the rows below, and Copy had
nothing to copy. A scroll view, or a list of texts inside one, worked.

The texts of a list's rows were registered with their area like any other: the walk saw them,
and the registry (`TextStop`) knew their identity and their box. But the shell then went to the
**tree** for everything else — a text's words, where a point fell among them, where a handle
hung — and a list's rows are not in the tree. They are built for the frame, walked, and let go
(milestone 592). Looked for by identity, a row was nowhere, and every question about it came back
empty.

## What was done

- **Each text registers what it answers for a selection, with its area.** A text inside a
  selection area hands the walk a `SelectableText` as it is painted — its words, where a point
  falls among them, where a handle hangs from them — and the registry keeps it with the text's
  box. Every question a selection asks is put to what the text registered, never to the tree:
  the press, the drag, the double click and the held word, the handles, Select all and Copy.
- **So a list built on demand takes part.** A press on a row of a `ListView::builder` begins a
  selection, a drag selects across the rows, a double click selects a word of one, and Copy gives
  their words, one row a line. A `GridView::builder` is a list of rows of cells, and its cells
  take part the same way, in reading order.
- **A drag held past the edge scrolls the list** (milestone 739), and the selection takes the
  rows the list builds as they come in: held below the first row, it scrolls to the last and
  selects all fifty.
- **A row recycled out of view stays selected.** The texts a selection is worked out over are
  remembered from its beginning (milestone 739); what they remember is now what each one
  registered, which outlives the row. A selection that began in a row the list has since let go
  of still begins there, and Copy still copies it.
- `frus-widgets` gained the **`SelectableText`** trait, the field `TextStop::text` that holds it,
  and the hook **`Widget::selectable_text`**, which `Text` and `RichText` answer. It replaces the
  three hooks the shell asked of a widget in the tree — `selection_text`, `selection_hit` and
  `selection_grip`. `TextStop` is no longer `Copy`.

## Decisions

**The texts register themselves, rather than the list keeping what it built.** Two fixes were
open. A list could keep the rows it last built, so that the tree — or something beside it — would
find them again; or each text could hand its area what a selection asks of it. The first keeps a
whole frame's worth of rows alive for the sake of a few questions about words, and would have to
be done again by every widget that builds as it lays out: a list, a grid, a paged view, a layout
builder. The second is how selection works across toolkits that do it well: the texts in an area
register with it, and the area talks to them, not to the tree. It also needs nothing new from the
lists: they already walk their rows through the same walk as everything else (milestone 592),
and the walk is where a text registers.

**What a text registers is apart from the widget.** A `SelectableText` holds the words and what
lays them out — the style a `Text` resolved, the runs of a `RichText` — not a reference to the
widget, which the frame drops. It is made once a frame, for the texts in an area that show, and
lays its words out only when it is asked a question, keeping the layout for the next question at
the same width: a drag asks once a move.

**A trait, not a type.** `Text` and `RichText` answer with the same small object, but a widget of
words of its own can implement `SelectableText` and take part on the same footing.

**One hook for three.** The old hooks answered a question each and could only be asked while the
widget was alive. The new one is asked once, while it is, and its answer answers the rest.

## Tests

- `frus-widgets`, `region.rs`: the rows of a `ListView::builder` in an area register their words
  in reading order, only the rows in view, and answer a hit and a handle after the list and its
  rows have been dropped; a text with no words registers nothing, and a hit counts characters,
  not bytes. The registry tests of milestone 575 pass on the new stop.
- `frus-widgets`, `richtext.rs`: the paragraph's hit and handle tests go through what it
  registers.
- `frus-shell`, through the driver with a mouse, over fifty rows of a `ListView::builder` in a
  200-px box: a press and a drag across three rows, then Ctrl+C, copies the three rows; a double
  click selects a word of a row; held 40 px below the list from the first row, the list scrolls
  at the selection's speed to its end, the first row is no longer built, and Copy gives all fifty
  rows from the first to the last; a selection begun in a row, carried down until the row has
  been recycled and let go inside the list, copies from that row to the one under the pointer,
  every row between in order. Over a two-column `GridView::builder`, a drag from the first cell
  to the fourth copies the four cells in reading order.
- The selection tests of milestones 575 to 578, 738 and 739 pass unchanged.
- fmt, clippy `-D warnings`, `cargo test -p frus-widgets -p frus-shell -p frus-demo` pass; the
  widgets gallery builds.

## Not done

- A row's own state is still not kept: a row that leaves and comes back is a new row with the same
  identity. A selection survives that because it is kept by identity, which a builder's rows keep
  as long as their index does; a list whose rows move to other indexes meanwhile (an insertion
  above) moves the selection with the indexes, not with the rows.
- Only a scrolling area **inside** the selection area, or the one it sits in, is scrolled; a
  selection area that holds several scrolling areas side by side scrolls the one the drag began
  in (milestone 739).
- Not run on a phone or in a browser.
