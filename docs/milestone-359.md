# Milestone 359 — A scroll that runs from the far end

Third of milestone 357's audit findings, and the one with teeth: `Scroll` had no
`reverse`. A conversation — the canonical use — could not be built.

## What reversing actually buys

Two things, and neither is cosmetic:

- content **shorter** than the viewport sits at the bottom rather than the top;
- the view **stays** at the end when content arrives.

The second is the whole point and it decides the design. Offsets have to be measured
**from the end an axis starts at**. A view resting at offset 0 is resting at the newest
message, and the newest message is wherever the end now happens to be — so appending one
does not move it. Numbering from the top instead would leave the view drifting away from
the end every time something arrived, which is precisely what this exists to prevent.

## One sign change, in one place

Measuring from the other end means the arithmetic between a finger and a number changes
sign. There are five places a screen delta becomes an offset — the wheel, the drag, the
release fling, each on two axes — and a minus sign at each is five chances to get it
wrong in a way only a device would show.

So there is one function instead:

```rust
pub fn offset_delta(&self, screen: (f32, f32)) -> (f32, f32)
```

on `Scrollable`, which every site now calls. Two sign changes with two reasons: the
content moves *opposite* the number (dragging down reveals what is above, a smaller
offset), and a reversed axis counts from the other end. A reversed area is right in all
five or in none — and it is unit-testable, which five scattered minus signs were not.
`Scrollable::content_origin` does the same job for the paint, and covers the short-content
case in the same expression.

**Nothing changes for the user's hand.** A finger pushes the content the way it moves, in
either direction; the scrollbar's thumb rests where the content is. `reverse` is invisible
except in the two behaviours above, which is the correct answer and the reference's.

## Scope

`reverse` applies to the axis the area scrolls along; a two-dimensional scroll takes it on
the vertical, which is the one a reversed view is ever about.

## Left

- **`List` and the virtualised list do not reverse.** They register their own scrollable
  areas, and a reversed list also has to *build* from the end — item 0 at the bottom —
  which is a change to the item walk rather than to the offsets. That is the other half of
  a chat view, and it is the next piece.
- **Pull-to-refresh above a reversed area** takes the top edge, where a reversed area's
  start is the bottom. The combination is odd enough that it is recorded rather than
  guessed at.
