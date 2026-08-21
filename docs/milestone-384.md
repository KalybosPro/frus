# Milestone 384 — A field whose text can sit somewhere else

`TextField` drew its value from the left edge of the content box and had no other option.

An amount field wants its figures on the right, where the decimal points line up down a
column. A code field wants them centred. Neither was reachable.

`text_align` is the builder, and `align_offset` is the whole of it: how far the alignment
pushes the text inside the box it was given.

## Nothing to distribute, nothing to push

The offset is zero unless the text is **narrower** than the box.

A right-aligned line longer than its field would otherwise be shoved off the left edge —
the one edge whose text must stay put, since that is where reading starts and where the
horizontal scroll brings the caret back to when you arrow to the beginning.

## One function, or the click stops matching the glyphs

The paint draws the selection, the text, the underline and the caret from a single
`text_x`. The hit test rebuilds the same geometry from scratch in `cursor_at`.

Those two have to agree. A caret placed from unaligned geometry on centred text appears
several characters from the tap — the kind of wrongness nobody reports precisely, because
it does not look like a bug, it looks like the field is broken.

So there is one `align_offset` and both call it. There is a test that taps a hair before the
first glyph and a hair after the last, for every alignment, and asserts the caret lands at 0
and at 2.

## It takes no direction, and that is the decision

The first version resolved `Start` and `End` against `theme.direction`, so a `Start` field
in an Arabic application would sit against the right edge.

`Widget::cursor_at` is handed a rectangle and nothing else. No theme, so no reading
direction. The paint could apply that push and the click could not — which would have put
the caret several characters from the tap in **every field of every right-to-left
application**, including every field that never asked to be aligned at all.

A push both sides can compute is worth more than one that is right in a place the other
cannot reach. `Start` is the left edge here and `End` the right; following the reading
direction belongs to the text layout one layer down, where the caret and the hit test
already live and cannot disagree with the glyphs.

That bug existed for one compile. It is written down because the next person to add a
geometry-dependent feature to a field will reach for the theme too.

## The placeholder moves with it

A centred field whose hint hugs the left edge jumps the moment the first key lands. The
placeholder is measured and pushed by the same function.

## Single-line only

A multi-line field stays at the start, whatever it is told.

Aligning wrapped text means moving **each line** by its own width, and the caret and the
click would then have to be told about an offset that differs line by line. That belongs
inside the text layout, which owns `caret_rect` and `hit_test`, and not in a widget nudging
a block sideways behind their backs. `align_offset` returns zero for a multi-line field and
says why.

## Left

`Justify` is treated as `Start`. Stretching the spaces between words is not something a
single line of input should do: it would move the glyphs under the caret every time a space
was typed. `TextAlignVertical` — the reference's other axis — is not here either.
