# Milestone 598 — LimitedBox, and "at most the room"

Milestone 579 left `LimitedBox` waiting on #52 with the `Aligned` factors. Milestone 597 did the
factors and named what was still missing: a measured box could hand its child the room, take it
away or give a number, but not say "at most the room". A paragraph under a width factor was one
line. Both needed that rule.

## What it does

`LimitedBox::new(child).max_width(w).max_height(h)` caps its child **only where nothing else
does**:

- Where the room on offer has no end, such as down a scroll or along a list, the child may take
  at most the limit. A child that asks for all the room is as big as the limit.
- Where the room has an end, the box changes nothing.
- The limit is a ceiling, not a size: a child that hugs its content still does.
- A paragraph wraps at the limit.

`Aligned` and `Center` with a factor now let their child take at most the room on that axis,
instead of asking it with no limit. A paragraph wraps at the room, as the reference's does, and
the box is the factor times the paragraph. This removes the difference milestone 597 noted.

## How

### Two new axis rules

`AxisConstraint` gains:

- **`Loose`**: the room on offer is the most the child may take, and it is not handed to it.
  With no room on offer, the child is as big as it likes.
- **`Limited(max)`**: as `Loose` where the room has an end. Where it has none, `max` is the
  most the child may take.

`AxisConstraint::offered` now says, for a bounded axis, whether the child is **handed** the
bound (a child with no size of its own takes it) or only **allowed** it. `AxisConstraint::at`
says the same for the walk.

### Filling each axis separately

- `Layout::compute_axes` in frus-layout takes free-or-bounded and filled-or-not **per axis**.
  `compute_scroll` is now a fixed choice of it: fill a lone bound.
- The walk's `Constraints` carry the same choice (`fill_each`).
- A box that hands its child the room on one axis and only allows it on the other can now say
  so. A factored `Aligned` is one: handed across, allowed down.

### At most means at most

A bound that is not filled is still the most the root may take. A container with no width of its
own around a paragraph comes out as wide as the paragraph on one line: taffy measures the text at
its natural width. `compute_axes` then **lays it out again at the bound**, where the paragraph
wraps. This is also the reference's answer: a paragraph that wraps is as wide as the width it
was allowed. A root with a size of its own keeps it, and spills.

### The walk and a factor below one

A factored box below one is smaller than its child. The walk allows the child what it had
again, which is the box divided back by the factor. Allowing it the box instead would have
wrapped the paragraph at half the width. At zero there is nothing to divide, so the child is
asked with no limit.

## Differences from the reference

- **What counts as "no end".** The reference's column gives a child no bound along its own axis.
  Here, a column that has a height of its own offers it. A `LimitedBox` in a bounded column
  therefore sees a bounded room and changes nothing, where the reference's would cap. Inside a
  scroll, the two agree.

## Found on the way

`SizedBox::expand` in a column inside a scroll is 0 px wide. It fills by a percentage of a
column that hugs its content, and so hugs nothing. It is left as it is here.

## Verification

- `LimitedBox`:
  - Down a scroll, a child that asks for all the room is 120 px tall at a 120-px limit, and as
    wide as the scroll.
  - On a page, the same child is the page.
  - A 40×20 block stays 40×20.
  - Along a horizontal scroll, the limit caps across.
  - A paragraph along a horizontal scroll is 120 px wide at a 120-px limit, and several lines
    tall.
- `Aligned`:
  - A paragraph under a width factor of 1 in a 120-px page wraps: several times a line's
    height.
  - Under a factor of 0.5, it still wraps at 120, not 60.
- The library's tests, the workspace's and the goldens.
- Mutations, each failing a test:
  - no limit where the room has no end;
  - the box not divided back by a factor below one;
  - every bounded axis handed rather than allowed;
  - the factors' axes asked with no limit, as in milestone 597.

  One survived and is equivalent: handing the child, in the walk, the extent a loose axis
  produced. That extent is the child's own size, so it takes the same size either way.

  Passing the child's request to fill up through a loose axis was tried and taken out. No test
  could tell it from its absence: the failure that had suggested it was `SizedBox::expand`'s,
  below.
