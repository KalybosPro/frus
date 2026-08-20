# Milestone 364 — A slider over a range, in steps, saying where it is

Found by widening milestone 357's depth audit to the selection controls. `Slider`'s whole
surface was

```
new  width  on_change  enabled
```

against the reference's `min`, `max`, `divisions`, `label`, and a keyboard.

## What a normalised slider costs

It was a `0.0..=1.0` control and nothing else. Every caller with a real range — a price
from 20 to 200, a font size from 8 to 72, a volume in decibels — had to divide on the way
in and multiply on the way out, in both the view and the update, with the two conversions
written far apart and only agreeing by luck. Nothing in the framework knew what the number
meant, so nothing could say it: the accessibility node reported a **percentage** of a range
the reader was never told.

`range(min, max)` puts the units back where the caller wrote them. `on_change` hands over
the real value, `Semantics::range` carries the real bounds, and a value label formats the
real number.

The default is `0.0..=1.0`, which is what the control did before — so no existing caller
changes, which is why the range is a builder rather than an argument to `new`.

## Divisions, and the arithmetic that follows

`divisions(n)` snaps to `min + k·(max−min)/n`. `RangeSlider` has had it since it was
written; the plain slider, the one anyone reaches for first, did not.

It also decides what an arrow key moves by — one division, or 5 % of the travel when the
travel is continuous. That is the reference's rule, and it falls out for free once the step
is a fraction rather than a number of units.

## It was not a keyboard control at all

`Slider` had no `focusable` and no `on_key`. Tab passed it by and the arrows did nothing —
on a control whose entire job is a value that arrows are the obvious way to move. Its own
`RangeThumb`, in the same file, has had both for milestones.

Both are here now, on the same terms as everything else in this framework: focusable only
when it is enabled *and* somebody is listening, and inert to a key arriving from a stale
focus after the caller froze it.

## Two things held rather than rejected

**A backwards range** (`range(200.0, 20.0)`) is sorted. Left alone it would be a silently
empty travel, which is a worse answer than the obvious one.

**A value outside the travel** is clamped, not asserted. A caller that lowers a ceiling
under a value it already had gets the ceiling — an app rebuilding its view from state it
is midway through editing should not panic.

## Left

`onChangeStart` / `onChangeEnd` — the two edges of a drag, which a scrubber wants so it can
pause a preview while the thumb is held. There is no drag-start or drag-end hook in the
widget trait at all, so it is a shell change as much as a widget one and wants its own
step. The colour overrides (`activeColor`, `thumbColor`, `SliderThemeData`) are the
selection controls' shared gap and want one step for all of them.
