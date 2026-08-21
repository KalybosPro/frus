# Milestone 387 — A bar wide enough to lose the selected tab in

Eight tabs on a phone stop being eight crushed columns when the bar scrolls, which
milestone 345 made possible. What it did not do is bring the selected tab back.

Select the ninth tab — from the application, from a restored session, from anything that
is not a tap on the tab itself — and the bar carried on showing the first three. The panel
below changed. The bar did not, and nothing said where the selection had gone.

The reference scrolls its bar to the selected tab. So does this now.

## A scroll region can be asked to keep something in view

`Widget::keep_visible` returns a box, in the widget's own coordinates, plus a **key**
naming which box it is.

The key is the whole mechanism. The box moves as the region scrolls, so a region that
chased the box itself would pin the content in place and no finger could ever move it —
the same trap `sync_pages` avoids by acting on a page **request** when it changes rather
than re-asserting it every frame. `Runtime::sync_visible` does the same with the key.

A test pins exactly that: the reader drags the bar somewhere else, the selection has not
changed, and nothing pulls it back.

## Recorded at rest, so the answer is absolute

`KeepVisible::rect` reaches the region in the frame's coordinates **as it would sit at
offset zero** — not where it is actually drawn.

This was a bug before it was a decision. A box recorded where it is drawn already has the
current offset baked into its position, so the offset that would centre it can only be
worked out *relative* to wherever the region happens to be. On the first frame, before
anything has been retained, there is no wherever to be relative to: the walk had already
placed the strip at the selected tab, so the box came back centred, the arithmetic
answered "nothing to do", and the offset was never retained. The frame after found no
offset and put the bar back at the start.

At rest, the answer is the same number on the first frame and the hundredth.

## Opening there, then gliding

The walk uses that offset directly when the region has never been scrolled: a bar restored
on its ninth tab shows the ninth tab rather than showing the first and then travelling.
The same answer a paged view gives its initial page.

`sync_visible` retains that offset on the first sighting — the jump is invisible, since the
frame was already drawn there, but without it the region would own no offset and the next
frame would have nothing to keep it. Every change after that is a scroll **target**, so the
bar glides.

## Centred, not merely inside

`Scrollable::centre` sits beside milestone 386's `reveal`, and the request picks.

A tab bar centres, which is the reference's behaviour and the readable one: a selected tab
flush against the window's edge reads as the end of the row when it is only the edge of the
window. The clamp is what keeps that honest — the first tab cannot be centred without
scrolling backwards, so it stays at the start, and the last stays at the end.

Keyboard traversal keeps `reveal`'s least-movement policy. Two policies, and the widget
that asks says which it means, because a form that re-centred on every Tab would be motion
nobody asked for.

## Riding on the region

The request travels **on** `Scrollable` rather than in a registry of its own.

A registry would have needed a slot in `BoundaryData`, in `Snapshot`, and at each capture
and replay site, and a repaint boundary that cached a tab bar would otherwise drop its
request on every hit. The regions already survive all of that, and the request belongs to
one anyway.

## Left

Only the generic scroll view reads the request. A virtualised list would need the item's
extent rather than a laid-out box, which it has, but nothing asks yet.
