# Milestone 385 — A page view's ends, its direction, and its right not to snap

`PageView` had one shape: pages flush from the leading edge, index 0 first, and a
release that always sprang to a boundary. Three properties were missing, and one of
them was not a missing option but a wrong default.

## The ends were not padded, and the last page could not be reached

Set `viewport_fraction` below one and you have a carousel: the neighbouring pages show
at the edges, which is how a view says "there is more this way" without a hint or an
arrow.

Ours opened with page 0 **flush against the leading edge**, all the slack on the other
side. The reference centres it, and centring is not decoration here.

Fifty pages at 0.8 of a 300 px viewport: each page is 240 px, so page 49 rests at
`49 × 240 = 11 760`. Unpadded, the content is `50 × 240 = 12 000` and the travel stops
at `12 000 − 300 = 11 700`. **Sixty pixels short of where the last page rests.** The
snap would aim at 11 760, the edge would refuse, and the spring would pull it back —
every time, for ever, on the last page of every carousel.

With `(viewport − extent) / 2` at each end the content is `count × extent + 2 × pad`
and the travel works out to `extent × (count − 1)` exactly, which *is* the last page's
resting offset. Every page reachable, and the arithmetic says so rather than the eye.

`pad_ends(false)` is the flush version, for a carousel that wants it. The test asserts
both: padded, `offset_of(49) == max_x`; unpadded, `offset_of(49) > max_x`.

At the default fraction of one the padding is nil — a page already fills the viewport —
so nothing that existed before this moves. The golden did not change.

## Reverse: not a mirror, an answer to which end index 0 is at

`ListView::reverse` had this and a page view did not, though it is the same question:
a walkthrough read right-to-left, a gallery whose newest picture is index 0.

The window arithmetic is untouched, and that is worth saying. A reversed view counts
its indices from the far end and a reversed offset counts its pixels from the same one,
so index and offset agree about which way forward is. Only where a page **lands**
differs, and that is one `match` on two lines.

The region carries `reverse_x` / `reverse_y` now instead of two hardcoded `false`s, so
the drag, the overscroll glow and the scroll-to-offset all read the axis the same way
round. Hardcoding those was the bug waiting to happen: a reversed view whose glow
flashed at the wrong edge would have been found on a device, not here.

## Snapping off, without losing the pages

`page_snapping(false)` is a scrollable that happens to know where its pages are.

The temptation was to drop the `PageSnap` from the region and be done. That would have
taken the pages with it: `on_page_changed` would go quiet and `page(3)` would stop
working, because both read the snap to know what a page even is. The reference keeps
the controller either way.

So the flag rides **on** the snap, and one place reads it — the release. Everything
else that asks "is this area paged?" still gets yes.

```rust
if let Some(snap) = area.page.filter(|snap| snap.snapping) {
```

The test pins the difference at the release and nowhere else: the same view, left a
fifth of a page along with no speed behind it, stays there when snapping is off and
springs back to page 0 when it is on. A second test walks the rest — the page reported,
the page requested — and finds it unchanged.

## Left

`allowImplicitScrolling` (building the neighbouring page so a screen reader can move to
it) waits on the semantics tree knowing about off-screen children. `clipBehavior` is
hard-edged here with no way to ask otherwise.
