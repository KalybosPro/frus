# Milestone 347 — Two warts in the text layer

Both were written down before they were fixed, which is the only reason either got fixed.
One was a note in milestone 346; the other had been pinned by a test on the roadmap since
milestone 343 said *this is a wart, and changing it should be a decision.*

## A limited paragraph was handed over as lines

Milestone 343 gave `Text` a line limit by asking the shaper where the lines broke, keeping
the first few and joining them back together with newlines.

Newlines are the wrong glue. A paragraph handed to the renderer as lines is a paragraph
**per line**, and every rule that spans a paragraph stops working — most visibly
justification, which leaves the *last* line ragged and can only do that if it knows which
line is the last. So `Text::styled(…).align(Justify).max_lines(2)` produced two ragged
lines, and 343 recorded the caveat rather than the fix.

Milestone 346 had to solve the same problem for rich text and could not use the same trick:
runs cannot be joined with newlines without inventing runs. It cut a **prefix** instead, at
a byte offset the shaper chose. That is the answer for both, and plain text uses it now:
`frus_text::line_spans` returns the visual lines as byte ranges rather than as strings, the
widget keeps `text[..spans[max].start]`, and the renderer breaks it exactly where it would
have broken it anyway.

The proof is in the golden: a paragraph that is justified *and* limited to two lines, first
line flush to both edges, last ragged and ending in an ellipsis. It could not be drawn a
milestone ago.

Nothing else moved. All ninety-one goldens agreed before and after, because the renderer
was always going to break the prefix in the same places.

## A collapsed box drew its whole label

`truncate` returned the string untouched when the box had **no room at all**, on the
reasoning that a zero width meant "the layout has not run yet" rather than "there is no
room". It came from the app bar, whose title room has a floor of 64 px and cannot produce a
zero in the first place.

What the exception actually did was let a genuinely collapsed box draw its whole label over
whatever was beside it — the one thing ellipsising exists to prevent. A box with no room
gets an ellipsis and nothing else now, and the test that pinned the old behaviour says so
in those words.

## Left

- **`line_spans` shapes.** Once per frame per limited text, and the renderer shapes it
  again immediately afterwards. Both places would like the shaped result; neither has a way
  to hand it to the other.
- **A limited text still measures its lines twice** — once for the height cap through the
  cached measurement, once for the spans.
