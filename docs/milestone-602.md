# Milestone 602 — Sticky section headers

Issue #29 asked for three results. Milestone 494 delivered the first, a header that collapses
as the page scrolls. It said the second, "a section header that sticks until the next pushes it
off", was expressible on `ScrollOverlay` and **not built**. This builds it.

## What it does

```rust
Flex::column()
    .child(StickyHeader::new(Text::new("A"), names_under_a))
    .child(StickyHeader::new(Text::new("B"), names_under_b))
```

inside a `SingleChildScrollView`, or as the rows of a `ListView`:

- At rest, each header is where a column would put it, above its section, and it takes that
  room.
- Scrolled into its section, the header **stays at the top** of what is visible, and the
  section goes under it.
- At the end of its section, the next section's header **pushes it off**, pixel for pixel, and
  then sticks in its turn.
- A stuck header is **drawn over** its section and **takes the taps** there. The part of the
  section under it does not.

It works down a vertical scroll. A horizontal one is not covered yet.

## Why not on `ScrollOverlay`

Milestone 494 said this could be built on `ScrollOverlay`: work out which section the offset
is in and draw that section's header over the list. That needs every section's position, and
an overlay is only handed the offset and its own box. The positions exist only in the layout,
which the overlay cannot read. An application would have had to know its section heights in
advance.

The walk already knows everything needed, and nothing new has to be passed around:

- **where the section is**: its box;
- **where the header was put**: its first child's box;
- **what is visible**: the clip it is walking under, which inside a scroll is the viewport.

The header's shift is the distance from where it was put to the top of what is visible. It is
never negative, so the header never goes above its section's top, and never more than the room
left above the section's bottom. That second limit is what pushes it off.

## How

- A new hook, `Widget::sticky_header`, which wrappers forward.
- In the walk, a section walks its header with the shift added, then its body.
- **To draw the header over the body**, what the header added is moved after what the body
  added. That covers its primitives and the registries a pointer is matched against from the
  last one back: taps, long presses, ink, hover and drags. **Focus order and semantics keep
  reading order**, header first. A screen reader reads the header before its section, and Tab
  reaches it first.
- A section is left out of the paint cache. Where it is painted depends on the scroll, not only
  on its layout.
- The layout is untouched: a sticky header takes the same room as a column's first child.

## Verification

- Two 120-px sections, each a 20-px header over a 100-px body, in a 200-px scroll:
  - at 0, the headers are at 0 and 120;
  - at 50, they are at 0, stuck, and 70;
  - at 110, the second pushes the first off: −10 and 10;
  - at 130, the first is at −30 and the second is stuck at 0.
- At 50, the body is painted before its stuck header. A tap at y = 10 reaches the header, and a
  tap at y = 40 reaches the body.
- The same sections as the rows of a `ListView`: at 50, the first header is at 0 and the second
  at 70.
- The sections in these tests sit behind a key, as a list's rows usually do.
- Mutations, each failing a test:
  - no shift;
  - no limit at the section's bottom;
  - the taps left in layout order;
  - the primitives left in layout order;
  - a key's wrapper not passing the question on. This one survived until the sections were
    put behind a key.
