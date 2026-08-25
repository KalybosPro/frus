# Milestone 409 — A line's height, said as a multiple

The reference's `TextStyle` carries `height`: the height of a line as a **multiple of the
font size**. Ours did not, and milestone 402 left a note saying the shape it would need was
now the shape that existed — every `TextStyle` field an `Option`, inheritance for free.

This adds it, and closes a duplication that had been waiting for someone to touch it.

## A ratio, not a length

`1.0` packs the lines to exactly the type's size; `1.6` opens a paragraph up. The reference
makes it a ratio and the reason is the reader: at a `height` of 1.5 a 20 px line is 30 px,
and when someone turns their font size up and that 20 becomes 40, the line becomes 60. A
length would have stayed at 30 and closed the paragraph up **exactly when it needed
opening**.

`the_leading_grows_with_the_reader` is that sentence as a test.

## Two constants that had to agree

`LINE_HEIGHT_FACTOR` existed twice: once in `frus-text`, which measures, and once in
`frus-gpu`, which paints. Both were 1.2, so nothing was broken — but it is the same shape as
the bugs milestones 407 and 408 spent themselves on, sitting quietly until someone changed
one of them.

`ResolvedTextStyle::line_height()` is now the one place the number is decided, and both
crates import the same `DEFAULT_LINE_HEIGHT` from `frus-core`. A measure and a paint
disagreeing about how tall a line is puts the second line of every paragraph where the
layout reserved nothing.

## The whole wire, or none of it

```
TextStyle::height  →  ResolvedTextStyle::line_height()
                          ├─ measure   (`measure_at`, and the cache key)
                          ├─ line_box  (the floor under a one-line box)
                          └─ paint     (`Metrics::new(size, size × height)`)
```

Adding the field to `Primitive::Text` turned every construction site into a compile error,
which is how milestone 406 found its forty-seven and how this one found its ten. A ratio is
unitless, so the scaling and fading transforms pass it through untouched.

**The cache would have been silently wrong.** `MeasureKey` recorded text, size, weight,
italic and width — not the line height. Two paragraphs of the same words at different
leadings would have shared one answer, and the second would have been quietly incorrect. The
key carries it now, and `two_line_heights_do_not_share_a_cached_answer` asks in both orders
so that a cache answering from the *wrong* entry fails as loudly as one that never filled.

## The tests

In `frus-core`: the ratio resolves against the size that asked for it; the leading grows
with the reader; it inherits like every other field and a nearer style still wins.

In `frus-text`: doubling the line height doubles the measured block and leaves the width
alone; two heights do not share a cache entry; a one-line box's floor follows the style's
height rather than its size.

## Left

- **`letterSpacing`, `wordSpacing` and `fontFeatures` are not reachable.** cosmic-text
  0.12's `Attrs` has no letter spacing at all, so these need a dependency bump before they
  can be anything but a field nobody reads — and a field nobody reads is worse than a
  missing one.
- **`fontFamily`** is reachable (`Attrs::family`) but collides with `family_for`, which
  picks a face by script because Android has no cross-family fallback. Naming a family has
  to compose with that rather than override it, and it deserves its own step.
- **`shadows` and `background`** are paint-level and need the renderer to draw behind and
  under a run, which it has no path for yet.
