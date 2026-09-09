# Milestone 496 — A picker wheel

Answers [#48](https://github.com/KalybosPro/frus/issues/48).

`ListWheel`: the scrolling cylinder of values you spin to pick one.

## The question the issue asked, answered plainly

> The perspective is a per-row transform, which `Transform` can already express — check
> whether it can do it without a full 3D matrix, and say so if it cannot.

**It cannot.** A row on a cylinder is tipped about a *horizontal* axis and then divided by
its depth, and what the paint applies to a subtree is an `Affine` — a 2D affine, which by
construction maps parallel lines to parallel lines. A tipped row's near edge is wider than
its far edge; that shape is a trapezoid, and no affine produces a trapezoid. Adding one
would mean a projective transform in the compositor, which is a change to `frus-gpu` and
not to a widget.

So this is **not** a cylinder, and the module says so at the top rather than in a footnote.
What it is, is every part of a cylinder a reader actually notices, each of which an affine
*can* express:

| what a cylinder does | how it is done here |
|---|---|
| rows pack together towards the ends | centres at `r·sin θ` rather than at the flat distance |
| rows squash | `scale_y = cos θ`, which is foreshortening exactly |
| rows recede | `scale_x = 1/(1 + p·z)`, a uniform narrowing rather than a taper |
| rows dim | `opacity = cos θ` |

The one part left out is the taper itself, and the golden is the argument that it is the
part nobody looks for. Anything past a **quarter turn** has gone over the horizon and is
not drawn at all — not faded to nothing, not drawn.

`opacity` and `scale_y` are the same number twice, and that is deliberate: the fade is
where the missing taper's share of the illusion goes, so it is tied to the foreshortening
rather than being a second dial to tune against it.

## Nothing new underneath it

The spin, the fling and the settling are the **paged scrollable's** (`PageView`), with each
row a page of its own height. A wheel therefore inherits, for free and without a second
implementation to keep in step:

- a release that springs to the nearest row instead of stopping between two;
- the virtualised window — a wheel of three thousand minutes costs what a wheel of ten
  does;
- opening on the row it was asked for, on the **first** frame rather than after one that
  showed row nought;
- the overscroll at the ends.

Two small things had to be added to make a page a row. `PagedView` gained an **extent in
pixels** beside its viewport fraction: a page is a share of the window it is read in, and a
row of a wheel is a line of text, which is not a share of anything. And `pad_ends` — which
already existed for carousels — is what lets row nought and the last row reach the middle
at all, the middle of a wheel being where the answer is.

## The window is wider than the viewport

This is the one place the wheel is not simply the paged view. On a flat strip the rows on
screen are the ones whose flat positions are inside the viewport. On a cylinder they are
not: a row a quarter turn away is compressed into the last pixels at the edge, and its
*flat* position is a quarter of a circumference outside. So the window is widened in both
directions by `r·π/2` worth of rows, and the ones that turn out to have gone over the
horizon are dropped — **before they are built**, since building them is the cost the
virtualisation exists to avoid.

Widening rather than recomputing keeps the existing arithmetic, reversal included,
untouched.

## Accessibility, which the issue said not to skip

A wheel is **not** a list to walk row by row. It is a one-dimensional selector, and it is
announced as one: the role a platform's own picker reports, the chosen value in the
caller's words, and the position in the set as the range — `0`, the index, the count.

That is also what makes the selection announced *as it changes*, and without a mechanism
this framework does not have: the value travels the path a slider's value travels, and a
reader hears the new one for the same reason they hear a slider's.

`ListWheel::label` is what supplies the words. Without it the value falls back to "31 of
60", which is a position rather than an answer to *what is this set to* — a test says so
out loud, so that nobody reads the fallback as good enough.

## What the wheel does not draw

**The band across the middle.** A wheel is a cylinder of rows; what marks the chosen one is
a decision about the screen it is on — a band here, two rules elsewhere, a tinted panel on
a dark page — and a widget that drew it would be a widget every one of those had to argue
with. It is a layer of a stack, in four lines, and both the golden and the demo are of that
composition rather than of something easier to render.

## Verification

Eight unit tests. The geometry alone at the three points worth pinning — face on, a third
of a radian round, and the horizon — plus that it is symmetric and that a **larger
diameter is a flatter wheel**, because that is the one parameter a caller will reach for
and a sign the wrong way round is worse than no parameter at all.

Then the wheel: the chosen row rests across the middle and the last row can be reached; a
release settles on a row **and on the row it was sent towards**, asked in both directions,
because a wheel that always settled on the same row would pass a one-directional test on
its own; three thousand rows build about a dozen, and more than a flat strip would.

And the test that catches the bug this framework has shipped before — **does the geometry
reach the paint?** It reads the transforms the layers carry: exactly one row is face on,
and nothing is ever stretched, a cylinder turning rows away from the reader and never
towards. Removing the one branch in `ui.rs` leaves the other seven green and fails that
one.

The golden is the rest of the argument, and it is the only thing that could be: whether an
affine's worth of a cylinder reads as a cylinder is not a question a number can answer.

## In the demo

The settings screen, beside the stepper, so the two can be read against each other: a
stepper is for a number you nudge, a wheel is for one you spin past sixty of.
