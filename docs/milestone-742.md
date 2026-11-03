# Milestone 742 — Aligned and cut texts selected where they are drawn

## Objective

The limit milestones 573 and 738 left in their "Not done": a selectable `Text` or `RichText`, or
one inside a `SelectionArea`, was laid out for its selection from the start of each line and
whole. A centred, end-aligned, justified or right-to-left line was highlighted where a
start-aligned one would have been, a press landed on the wrong letters, the handles hung in the
wrong place, and a text a line limit or an ellipsis cut short was highlighted over lines it does
not draw.

## What was done

- **The selection's layout is the drawn one.** A text's hits, highlight, handles and bar anchor
  are worked out from the words in the box the paint hands the renderer: its width, whether it
  wraps there, and its alignment. Centred, end-aligned, justified, left and right lines are hit
  and highlighted where their glyphs are, for a selectable `Text` or `RichText` and for either in
  a `SelectionArea` — a press, a double click, a drag, the caret keys and the handles alike.
- **Right-to-left lines read from the right.** A character's box and its caret stops now follow
  the direction of the glyph that draws it: a right-to-left character starts at its right edge,
  so the first caret of an Arabic line is at the right, a press there is its start, and a line
  that mixes directions is highlighted as one box per stretch on screen rather than one box from
  the first selected character to the last.
- **A cut text is selected as far as it is drawn.** For a text a line limit cuts short, with or
  without an ellipsis, the layout is of the words kept, and its indices are still the whole
  text's: a selection that reaches into the hidden words is highlighted to the end of what is
  drawn, the ellipsis included; a caret or a handle past the cut stands after the last drawn
  character; and a hit past the end of what is drawn — on the far half of the ellipsis, or below
  the last line — is the end of the words, so a drag to the end of a cut paragraph selects the
  whole of it. **Copy takes the whole selected words**, the hidden ones too.
- **A highlight does not run past a clipped box.** Words a box cuts off at its edge (a line that
  may not wrap, with `TextOverflow::Clip` or `Fade`) have their highlight cut at the same edge.
- **A rich paragraph's ellipsis keeps it to its limit.** Its last run was ellipsised to the
  box's width on its own, though it shares its line with the runs before it; with them it could
  still be too wide and wrap onto a line past the limit, drawn below the box. The kept runs are
  now shortened a character at a time until they fit the limit. Found by this milestone's test:
  the highlight, following the glyphs, came out on two lines.
- `frus-text`: **`TextLayout::in_block(text, style, block)`** and
  **`TextLayout::of_runs_in(runs, block)`** lay a text out in a `TextBlock`, as the renderer
  does; **`TextLayout::elided(visible, source)`** says the drawn text is a cut of a longer one.
  `TextLayout::resolved` and `TextLayout::of_runs` are the start-aligned cases of the two.

## Decisions

**One layout, the renderer's.** The alignment is not added afterwards as an offset per line: the
layout is shaped with the alignment set on its lines exactly as the renderer sets it, and the
glyph positions it reads already carry it. An offset worked out on the side would have had to
know how a justified line spreads its spaces and where a right-to-left line starts in its box —
the shaper already knows both, and the highlight cannot drift from the glyphs if it is read from
them.

**What Copy gives for a cut text: all of it.** An ellipsis says words are missing, not that they
are gone; a reader who selects a cut label and copies it wants the label. So a selection's ends
are indices into the whole text, Select all selects all of it, and a drag that runs off the end
of what is drawn selects to the end of the words. Only the highlight is limited to what can be
seen, and the ellipsis is highlighted when the selection covers anything it stands for, so that
something on screen says the hidden part is selected.

**Caret stops from the leading edge.** Each character now has a box (its left and right edge) and
each boundary a caret stop at the leading edge of the character after it, or the trailing edge
of the one before at the end of a line. For a left-to-right line this is exactly the old offset
list, so a field, which uses the same layout, is unchanged: the plain layout's tests and the
fields' pass as they were.

**Nothing in the shell changed.** A selection area already asked the widget where a press lands,
where a handle hangs and what to highlight (milestone 575); answering those questions from the
drawn layout was all the fix needed.

## Tests

- `frus-text`: a centred line's first caret is half the room left over in from the edge, its
  highlight covers the word there and a press at the middle of the box lands in the middle of the
  word; an end-aligned line ends at the box's edge, wrapped or not; a justified paragraph's first
  line is highlighted from edge to edge and its last line is ragged; an Arabic line's carets run
  leftwards from the right edge, its first word is highlighted at the right, and caret and hit
  round-trip; a cut text answers in its source's indices (the ellipsis highlighted for a hidden
  range, a caret past the cut after it, a hit past the end or below it the source's end); a
  start-aligned block gives the plain layout's carets.
- `frus-widgets`, `text.rs`: a centred selectable text is highlighted, hit and given handles in
  the middle of its box; an end-aligned one is highlighted against the right edge, wrapped or
  not; a justified line edge to edge; a right-to-left text from the right; a text cut to one line
  with an ellipsis is highlighted on that line only and as wide as what is drawn, a hit below it
  or past the ellipsis is its end, and its own selection copies every word — an area's range too;
  a clipped line keeps its highlight in its box.
- `frus-widgets`, `richtext.rs`: a centred paragraph's bold run is highlighted, hit and given a
  handle where it is drawn; an end-aligned one ends at the edge; a paragraph cut to one line is
  highlighted on that line, a hit below it is its end, and Copy takes all its words.
- `frus-shell`, through the driver with a mouse: in a selection area, a drag over a word of a
  centred line copies `words` and paints its highlight in the middle of the box; a drag over the
  last word of an end-aligned line copies `here`, highlighted against the right edge; a drag
  across a text cut by an ellipsis copies the whole sentence and highlights one line; a centred
  text selectable on its own copies the word dragged over with Ctrl+C.
- fmt, clippy `-D warnings`, `cargo test -p frus-text -p frus-widgets -p frus-shell -p frus-demo`.

## Not done

- **A line that mixes directions** is highlighted correctly, one box per stretch, but its caret
  at the boundary between two directions stands at one of the two places it could, and a press
  there picks the nearer stop; a field's bidirectional editing was not looked at.
- **A soft-wrapped right-to-left paragraph** was tested only on one line.
- A `RichText` that does not wrap and runs past its box is still not ellipsised (it never was),
  so its highlight is only clipped.
- Not run on a phone or in a browser. Goldens were not run: no golden holds a selection.
