# Milestone 341 — The line text sits on

The last two of the twenty-two widgets milestone 336 counted, and the catalogue closes.

But `Baseline` and `IgnoreBaseline` on their own would be a pair of ornaments. What makes
them mean anything is the thing they adjust and which did not exist here: **baseline
alignment in a row**. A price beside its currency, a figure beside its unit, a heading
beside the note next to it — top-aligned the big one hangs, bottom-aligned it hangs the
other way, centre-aligned neither sits anywhere a reader recognises. Only a baseline puts
them on one line.

## The number, and where it has to come from

A baseline is the distance from the top of a line box down to the line the letters sit on.
It is a property of **the font at a size**, not of what is written in it, and it is not
derivable from the point size: the ascent belongs to the face the fallback chain actually
chose, and guessing costs a few per cent per family — enough to see, in the only situation
this number is ever used in.

So `frus-text` shapes for it, and takes `line_y` from the same layout run the renderer
takes it from. That is what keeps layout and paint talking about the same line. It is
memoised on `(size, resolved weight, resolved style)`, which is everything it depends on.

## Why taffy cannot do this

taffy has baseline alignment, and it is unreachable from here. Its measure function asks a
leaf for a **size** and a leaf can only answer with one; there is no way to hand back an
ascent. Its own baselines therefore come from nested flex containers, and a leaf falls
back to its bottom edge — which for a piece of text is precisely the wrong answer.

The measurement has to come from up where the widgets are. So it does: a row with
`Align::Baseline` measures each child's natural baseline, takes the deepest, and turns the
difference into a **top margin** on each child. By the time taffy sees the row the children
are already where they belong, and `Align::Baseline` maps to a start alignment — it must,
because stretching would give every child the row's height and there would be no baseline
left to align.

The **natural** baseline is the right one to measure even though the children will be laid
out at some other width. Narrowing a piece of text adds lines below; it does not move the
first one.

## Baselines ride on the layout tree

A leaf's baseline is stored as the layout node's data, and `natural_baseline` reads the
first one out of the computed rectangles.

The alternative was a second walk of the widget tree alongside the rectangles, and it would
have been a copy of `build_layout` waiting to drift out of step: a scrollable, a stack, a
page view and a fitter are all **leaves** in that tree with their contents laid out
elsewhere, and a walk that got that wrong would read baselines out of boxes that are not
there. Only the walk that built the tree knows which branch it took, so the answer is
recorded as it goes.

`IgnoreBaseline` is a flag narrowed once at the top of that walk. Everything below keeps
its baseline and loses the right to be seen, which is exactly what the widget says.

## `Baseline`

Top padding, computed once the child's own baseline is known — the same trick `Intrinsic`
and `RotatedBox` already use: measure the child, write the answer into this node's style,
let taffy do the rest.

Asked for a line the child is already past, there is nowhere to push it up to and the child
is top-aligned instead. That is what the reference's documentation promises, and a test
says it in those words.

There is **no choice of baseline kind**. The reference takes an alphabetic-or-ideographic
parameter; here the text engine reports the alphabetic one, and offering a second name that
resolved to the same number would be worse than not offering it.

## Left

- **The catalogue is closed.** All twenty-two.
- **Ideographic baselines.** A different number, needing per-script metrics the text layer
  does not expose. Named here so it is a decision and not an oversight.
- **A baseline row measures each child on its own.** One natural layout per child, on a
  relayout-cache miss. Rows asking for baseline alignment are few and short, and the cache
  above absorbs the rest, but it is a linear cost where the other alignments are free.
- **Only the first baseline.** A child with several lines aligns on its first, which is
  what the reference does and what a row means; a `lastBaseline` has no caller yet.
