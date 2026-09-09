# Milestone 494 — A header that collapses as the page scrolls

Answers [#29](https://github.com/KalybosPro/frus/issues/29).

`ScrollOverlay`, and `CollapsingHeader` on top of it.

## The design question, answered

The issue put two shapes and asked for a decision rather than code:

1. **The protocol** — a viewport laying out a sequence of scrollable pieces, each given a
   remaining extent and each reporting what it consumed. Honest, composes, and everything
   asked for becomes a small widget over it. Also much the larger piece of work.
2. **A collapsing-header widget** — one widget holding a header and a scrollable body,
   interpolating the first from the second. A tenth of the work, and "it will not compose
   with anything".

**Neither, and the reason is that the second one's real defect is not its size.** A
widget that holds a header and a body is a widget that cannot be asked for a rule that
fills, a shadow that appears, a title that changes, or any of the other things that react
to an offset — every one of those would be another widget with another body inside it. The
composition failure is what makes it a down payment, not the line count.

So the thing built first is the **primitive the composition failure points at**:

**`ScrollOverlay`** — a scroll region with something drawn over it, built from where that
region has got to. Any body, any overlay, and the overlay is ordinary widgets:

```rust
ScrollOverlay::new(list, |(_, y), box_| header(y, box_.width))
```

`CollapsingHeader` is then thirty lines over it, and so is a progress rule, and so is a
shadow. That is the property option 1 was praised for, at option 2's cost.

**What it does not buy is the third result** — a page that is a list, then a grid, then a
list, scrolling as one. That still wants the protocol, and nothing here is a step towards
it. The roadmap says so.

## Identity: the region is the child

Both halves of milestone 493 name a scroll region by **key**, because a command written in
`update` has no other way to say which list it means. An overlay does: it is drawn over one
particular region, so that region is its **child**, and the offset it reads is
`child(0)`'s. Nothing is named, looked up, or kept in step.

A builder floating free of the region it watches would need a key, a lookup, and an order
the walk happens to run in — three things that can go stale without saying so, for a
generality nobody asked for.

## Why not a message, since `on_scroll` exists

Milestone 493 landed `Widget::on_scroll` the day before this, and an application could
keep the offset in its state and build a header from it. It would work and it would be
wrong here: the offset goes out as a message, comes back as a rebuild of the **whole**
view, and arrives a frame late — a header lagging the list it is attached to, at every
frame of every fling.

The two are not substitutes and the division is worth stating:

- **`on_scroll` is for what changes the application's state** — a load-more, a remembered
  position. It carries the extents as well, it costs a message, and what it changes is
  true *between* frames.
- **`ScrollOverlay` is for what changes only the picture.** Handed the offset inside the
  frame that draws it: nothing lags, nothing round-trips.

## What the builder is handed, and what it is not

**The offset, and its own box** — the same box a `LayoutBuilder` is handed, and for the
same reason. That second half was not in the first draft of this, and the golden is what
put it there: a header left to size itself is as wide as its own title, so the first
picture of this widget was a block of text with the list showing either side of it. Every
unit test passed, because they all measured heights. `HeaderState` carries the width for
the same reason, and a header states it.

**Not the extents**: how far the content can scroll is a fact about a layout that has not
finished — the builder runs inside it — and a number handed over here would be the
previous frame's without being able to say so. An overlay that needs "how near the end are
we" reads it through `on_scroll`, which is measured.

The price is `LayoutBuilder`'s, and it is the same price: the overlay is **rebuilt every
frame the region moves**, so it holds no retained state. Hover and clicks work; a menu in
the overlay would close itself. The body is an ordinary child and keeps everything.

## Where the header's room comes from

**The body's own top padding.** A scroll region's padding is already *inside* the viewport
and already scrolls with the content — the reference's `SliverPadding`, and the reason a
feed's last row can clear a floating button. So the room the header needs is the room the
list was going to reserve anyway:

```rust
ListView::new(…).padding_each(220.0, 0.0, 0.0, 0.0)
```

One number, in the place that already means "room at the top of this list", read by the
header rather than told to it twice. Two numbers that have to agree are two numbers that
eventually do not.

## The header gives way pixel for pixel

Not on a curve. A header moving at some other rate than the content under it reads as two
pages sliding over each other, which is a thing no interface should do by accident. It
stops at `collapsed_height` and pins there, or — `floating()` — goes entirely, for a
reading view where the chrome is worth its room only while somebody is choosing what to
read.

What a header is written against is the **fraction**, `0` open to `1` collapsed, not the
height: a size or an opacity interpolated on it reads the same whatever the two heights
are, and survives somebody changing them.

## In the demo

The licence screen — four hundred and forty-two packages, the longest list here, and so
the one where a bar that keeps its whole height for ever costs the most. The title starts
half again as big and shrinks to a toolbar's, the subtitle goes first, and a rule appears
under the bar only once something has passed beneath it.

## Verification

Three tests on the primitive: that the overlay is built from the body's live offset, that
the body is still an ordinary scroll region registered in its own right, and that the
overlay is drawn **after** every row — which is also what makes a button in it clickable,
the hit registry being read last-first.

Six on the composition: the expanded height comes from the body's padding; it gives way
pixel for pixel and then stops; a floating header goes entirely and then stops being drawn
at all; the fraction is a real fraction of the range; a collapsed height taller than the
expanded one is held to it rather than collapsing upwards; and **the header is a band
across the whole page**, which is the one the pictures had to teach.

Two goldens, both ends of the travel: open, with the title at its full size and the
subtitle under it; and collapsed, with the title at a toolbar's size, the subtitle gone, a
rule under the bar, and the rows passing beneath it. **They were wrong twice before they
were right**, and neither error was visible from the numbers.
