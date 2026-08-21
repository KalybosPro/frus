# Milestone 386 — Tab could not reach what was scrolled out of sight

A form taller than its window had most of its fields **unreachable from the keyboard**.

Not hard to reach. Unreachable. Tab walked the two and a half fields on screen and
wrapped round to the first one.

## Registering what the eye can see

The walk records a focus stop inside a guard:

```rust
let visible = draw_rect.intersect(clip);
if visible.width > 0.0 && visible.height > 0.0 {
    // hits, long presses, drag sources … and focus stops
}
```

That guard is right for a **click**: a tap on empty space must not land on something
scrolled away under it. It is wrong for focus. The reference keeps its focus nodes
whether or not any pixels of them are on screen, and scrolls them into view when they
are focused.

So the focus registration moved out of the guard, and only the focus registration.

## Not "always", though — "when something could reveal it"

Registering every clipped widget would trade one bug for a worse one. A widget clipped
away by something that does not scroll cannot be brought into view by anything, so focus
would land where the eye can never follow: no ring, no caret, no way back except Tab
again and hope.

The condition is therefore about **rescue**, not visibility:

```rust
self.scroll_host.is_some() || (visible.width > 0.0 && visible.height > 0.0)
```

Inside a scroll region, a stop counts whether or not it is showing, because the scroll
can bring it in. Clipped by anything else, it is gone.

`scroll_host` is threaded through the walk the way `refresh_host` already was — replaced
on the way into a region, restored on the way out, so siblings do not inherit each
other's.

## Two boxes, because they are two questions

`Focusable` now carries both:

- `rect` — the **visible** box, clipped. Empty for a stop out of sight. This is what a
  click tests against, so `focus_hit` is unchanged and a tap past the fold still hits
  nothing.
- `bounds` — the box **unclipped**. Where the widget actually is. Arrow navigation places
  candidates by it, and the reveal works out what to scroll from it.

Collapsing the two would have broken one of them. Keeping only the clipped box makes an
off-screen stop a zero-size rectangle at the origin, which arrow navigation would then
treat as living in the top-left corner. Keeping only the unclipped one makes a click on
empty space focus whatever is scrolled underneath.

## Moving as little as possible

`Scrollable::reveal(target, current)` returns the offset that brings a box inside the
viewport, and how far the content moves to get there.

The **least** movement that does it. Centring the target instead would make Tab through a
form scroll on every single stop, including the ones already in the middle of the window —
motion nobody asked for, and the reference's default alignment is the leading edge too.

A target **too big** for the viewport gets its start edge aligned, and that falls out of
asking about the near edge first rather than being a case of its own.

Both numbers come back because they are not the same number: the offset is clamped to
what the region can actually do, and an outer region has to be told how far the target
**really** moved, not how far it was asked to move.

The reversed axis is not a second branch. `offset_delta` already turns a movement of the
content into a movement of the offset, and it is its own inverse — it negates or it does
not — so the same function turns the offset that was allowed back into the content
movement it bought.

## Nested regions, from the inside out

A sideways strip inside a page that scrolls down is two regions, and both have to move.

Identities here are hashes and carry no ancestry, so `Scrollable` records its own `host`
during the walk, exactly as the focus stops do. `Ui::reveal` follows that chain outwards,
and each region is told where the target **ended up** rather than where it started —
asking both about the original box would have them fight over the same pixels and land
somewhere neither wanted.

An empty answer means nothing needs to move, so no caller has to ask twice.

## Where the shell calls it

Only where the **keyboard** moves the focus: Tab, Shift+Tab, and the arrow-key
directional traversal.

A click already landed on something the reader could see. And a focus restored because an
overlay closed should put the page back where it was left, not chase whatever the
restoration happened to pick.

The glide is a scroll **target**, not a jump — the same easing a scrollbar drag or a page
request uses — so Tab through a form reads as one movement rather than a series of cuts.
Anything already moving that offset is let go of: the keyboard has just overruled it.

## Left

A stop inside a **virtualised** list that has not been built is still not a stop, because
nothing exists to focus. Reaching those means the list knowing its item count is its focus
count, which is a different piece of work.

`ensureVisible`'s `alignment` and `alignmentPolicy` are not exposed: there is one policy
here, the least movement, and it is the one keyboard traversal wants.
