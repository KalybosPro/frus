# Milestone 497 — Half a pixel, made impossible to get wrong

Answers [#54](https://github.com/KalybosPro/frus/issues/54).

`frus_core::fits`, a debug assertion where every measurement passes, and the four places
that were already breaking the rule.

## The decision, and it goes against the issue

The issue offered two shapes and called the first one right:

1. **Stop rounding.** Fractional boxes, rounded at the point of painting — what the
   reference does. "This removes the class of bug, and it is the larger change."
2. **Keep rounding, and make the rule impossible to get wrong.** "Cheap and honest."

**The second, and not because it is cheaper.** Option 1 turned out to cost one line —
taffy rounds by default and `disable_rounding()` turns it off — so the question was never
how much work it is to write. It is what it costs to *have*, and that is measurable.

Two fills of different colours abutting on a shared edge, over a green background, at a
height that splits into halves:

| | pixel at the seam |
|---|---|
| boxes rounded | `(255, 0, 0)` — red, then blue. Nothing between them. |
| boxes fractional | `(136, 187, 136)` — **a bright green hairline** |

Both fills cover half of the shared pixel and are composited in turn, so what comes out is
a quarter background, a quarter of the first and a half of the second. In an interface
with no green anywhere, a green line. This is milestone 289's own aside — *"rounding is
defensible, it keeps adjacent boxes from showing hairline seams"* — turned from a remark
into a number.

The reference gets away with fractional boxes because it composites differently. A
framework that paints independent anti-aliased quads over a background does not, and
saying so out loud is better than inheriting the reference's answer to a question its
renderer answers and ours does not.

That measurement is now [`crates/frus-test/tests/rounding.rs`](../crates/frus-test/tests/rounding.rs),
so the argument is a test rather than a paragraph, and turning the rounding off breaks it.

## The rule, in one place

```rust
/// The rule for anything that measures itself: report a whole number of pixels,
/// rounded up.
pub fn fits(extent: f32) -> f32 { extent.ceil() }
```

`ceil` and not `round`, because rounding to nearest is the bug it exists to prevent and is
what a new measurer writes by reflex. `frus-text`'s two existing ceilings now go through
it, which is what makes it *the* helper rather than a third copy of the habit.

## And a debug assertion where every measurement passes

Every measured leaf's closure is called from one place in `frus-layout`, so the rule is
checked there:

> a measured height of 28.8 is not a whole number of pixels, so the box it is rounded to
> is one it no longer fits in — go through `frus_core::fits`

**Two things it deliberately does not check**, and both were learnt by trying:

- **A measurement larger than the space offered is not a violation.** The issue asked for
  exactly this assertion; it fired on twenty-three tests at once. This framework lets
  content overflow and *reports* it (`Overflowing`, milestones 327–334) rather than
  crushing it, so a measurer answering with its natural size is doing its job. A box too
  small for it is a fact about the layout, not about the measurement.
- **A measurement that came back exactly at its bound is exempt.** It was clamped there,
  the bound is the caller's number rather than the measurer's, and rounding it up would
  hand back a box a fraction wider than the space offered — a different bug. Clamp
  **after** rounding up, never before.

An assertion that fired on everything would be turned off within a week, so both
exemptions have a test of their own saying why they are there.

## What the rule found the moment it was armed

Four places, all of them the same shape as milestone 289 and none of them noticed in the
eight milestones since:

- **`measure_wrapped` never rounded its height.** 289 fixed the width the wrapping bug
  presented on and left the other axis. A line height is a fraction of a size — 14.4 for a
  12 px line — so a two-line paragraph measured itself at **28.8**, and at a top edge that
  rounded the other way was handed 28. The same bug, in the same function, on the axis
  nobody looked at.
- **`measure_runs_wrapped`** — rich text — the same, for the same reason.
- **An empty label reserved a raw line height**, 19.2 for a 16 px label, and was handed 19.
  That is where a caret goes and where text put there later has to fit.
- **`Text` and `RichText` clamped to `max_lines × line_height`**, which is fractional by
  construction. Three allowed lines at 12 px come to 43.2, and a box of 43 holds two and a
  bit.

Every one of those would have presented as a clipped glyph or a line that vanished, and
none of them as arithmetic.

## What it costs to have fixed them

**Thirty goldens move, each by one pixel**, and always the same way: a box is a pixel
taller because the text in it was asking for a pixel more than it was given. Nothing is
laid out differently, nothing is lost, nothing is gained but the pixel that was always due.
The diffs were looked at, not counted.

A trap found on the way, worth recording: **`FRUS_UPDATE_GOLDENS=1` rewrites every golden
it renders**, not only the ones that differ. Running it over the whole suite touched a
hundred files, of which thirty had changed. Goldens have to be blessed **by name**.

## Verification

Four tests on the rule itself: a measurer that reports `146.4` — the number from 289 —
panics; one that goes through `fits` does not; a measurement clamped to a fractional bound
is not a violation; a measurement bigger than the offer is reported and not forbidden. The
pair matters as much as the assertion: an exemption without a test is an exemption
somebody deletes.

Two tests on why the rounding stays, which are the argument above with a GPU behind it.

One on the text measurer's height, across four sizes and three widths, and on the empty
label — mirroring the width test 289 left, on the axis it did not cover.
