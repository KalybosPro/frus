# Milestone 356 — As many columns as fit

The reference has two grid delegates. Milestone 353 did the first — a fixed column count,
with a shape for the tiles. This is the other one, and it is the one a real photo or card
grid uses: you give it a **maximum tile width** and it works the count out from the room
there is.

```dart
int crossAxisCount = (constraints.crossAxisExtent / (maxCrossAxisExtent + crossAxisSpacing)).ceil();
crossAxisCount = math.max(1, crossAxisCount);
final double usable = math.max(0.0, constraints.crossAxisExtent - crossAxisSpacing * (crossAxisCount - 1));
final double childCrossAxisExtent = usable / crossAxisCount;
```

Four columns of 125 in a 500 px grid at `extent(150.0)` — as many columns as it takes for
none of them to exceed the maximum.

## Why it is not a style

The obvious implementation is CSS: `repeat(auto-fill, minmax(150px, 1fr))`, which the
layout engine supports and which reads like the same idea. It is not the same idea.
`auto-fill` fits as many tracks of **at least** the given extent as it can —
`floor(500 / 150)` = 3 columns of 166 — where the reference's arithmetic gives tracks of
**at most** it. One column out, and in the direction that matters: the tiles come out
bigger than the maximum you set.

So the count has to be computed, and computing it needs the width, and the width does not
exist until the layout has run. That is exactly the wall milestone 353 stopped at.

Milestone 355 took the wall down: a `LayoutBuilder` is measured during the computation
and is as big as what it built. `Grid::extent` is one, underneath.

## What that costs the API

A grid that builds late cannot hold its cells: there is no grid to put them in until the
width is known. So the cells come from a **factory**, indexed, the way a list's rows do:

```rust
Grid::extent(160.0, photos.len(), move |i| photo_tile(&photos[i]))
    .gap(8.0)
    .aspect(1.0)
```

`Grid::new` is unchanged and still takes its cells one at a time. The two share every
builder method — the spacings, the shape — and `Grid::extent` passes them into the grid
it builds.

The caveats are `LayoutBuilder`'s, and they are real: the factory runs more than once a
frame, so it must be cheap and free of side effects, and the cells have **no retained
state**. Hover and clicks work; persistent keyboard focus and deferred overlays do not.
A grid of images or of buttons is fine. A grid of text fields wants `Grid::new` and a
count the application chose.

## What the tests found

The first run put every tile at **zero width**, four of them stacked at the left edge.

A `LayoutBuilder`'s content was laid out under `Constraints::definite` — *constrained*,
so it could not exceed the box, but free to be smaller — and a grid whose own width is
`Auto` hugged its content, which for a grid of `1fr` tracks is nothing at all.

This is the third time the same distinction has come up, and it is worth naming for the
fourth: **a box the content is handed is not a box the content is asked about.** A list's
items were being asked in milestone 351. A paged view's pages had been handed theirs
since long before. A `LayoutBuilder`'s content is the clearest case of all — it was
*built from* that box — and it was still being asked. It is handed now, in both paths:
the measurement fills the axes taffy offered and leaves the one it is asking about free,
and the paint fills the resolved box.

Nothing else moved: 795 unit tests and 91 goldens.

## Left

- **No spans.** A cell occupies one track. The reference's `GridView` has no spans either
  — that is `Table`'s business, or a custom delegate's — so this is not a deviation, but
  the layout engine can do it and applications will ask.
- **A grid that builds late is not layout-cached**, for the reason milestone 355 gives:
  a closure has no fingerprint. For a grid of a few hundred tiles that is a taffy pass a
  frame that a fixed-count grid would have skipped.
