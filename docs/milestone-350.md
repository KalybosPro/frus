# Milestone 350 — A layer goes where it was put

A layer — a group opacity, a fade, a rotation, a clip that is not a rectangle — is
rendered flat into a texture of its own and composited afterwards. That is what makes one
pass fade or transform a whole subtree without any primitive inside it knowing.

"Afterwards" was doing too much work. Every layer was composited after **all** of the
frame's ordinary content, so a group that something had covered came back out on top of
it. The scene said *group, then a rectangle over it*; the screen showed the group.

Found on a device at the end of milestone 349: the demo's translucent square, which
belongs to the home screen, painted over the Kanban board that had replaced it — and thin
slivers of a swipe background sitting at the left edge of every screen. Nothing in
milestone 349 caused it; it had been there for as long as layers have.

## Why no test caught it

Ninety-one goldens and not one of them covers it, which is worth saying plainly rather
than quietly fixing. Every fixture that draws a group draws it **last**, because that is
how a widget tree usually comes out: a card fades, an overlay floats, a dialog sits on
top. The case that breaks is a group with something *painted over it later*, and until a
navigator started keeping the page underneath alive for the back gesture, nothing in this
framework produced one.

`crates/frus-test/tests/layer_order.rs` produces four now, at the scene level where the
question actually lives: a group covered afterwards, a group covering what came before,
two groups sandwiching a rectangle, and a **masked** group covered afterwards — because a
fade is a layer too and would otherwise have been fixed by accident rather than on
purpose.

## The fix is where the ordering already lived

The batch planner (milestone 291) exists to answer exactly this question for content: it
gives every primitive a *level* from what it covers among the primitives before it, and
draws level by level. It skipped layers entirely — they were "composited separately", and
separately turned out to mean *later*.

So a layer is a batch now, of a kind of its own that never merges with another, with a
footprint of what its contents cover: their union, bounded by the layer's clip, and put
through its transform when it has one. Four corners through the matrix rather than two,
because two corners are only a bounding box when the matrix is axis-aligned, and a
rotation is exactly the case where a layer's footprint is bigger than the rectangle it
came from.

Then the render pass walks the batches in order and issues either a content draw or a
composite draw. That is the whole of it: no new pass, no new texture, no sorting. Layers
and content interleave inside the single render pass they always shared.

Two places needed the same treatment, and the second is the one it would have been easy to
miss: **nested** layers, composited inside a group's own pre-pass, were drawn after that
group's content for the same reason. A fade inside a card, over a rectangle inside the
same card, had the same bug one level down.

## What it cost

Nothing that shows. All ninety-one goldens are unchanged, and so is the rest of the
workspace: the ordering only differs where something was covered, and nothing that was
right before was covered.

The one real cost is batching. A layer whose footprint overlaps later content now pushes
that content to a level above it, which is a batch it cannot join. That is the same trade
the planner already makes for every other kind, and it is the trade that makes the picture
right.

## Confirmed where it was found

`XMJNW19B23011768`, release build, the Kanban board: the translucent square is gone, and
so are the slivers at the left edge. The bug was found by looking at a screen and it is
closed by looking at the same screen.

## Left

- **The backdrop path now cuts the frame at a batch** rather than at a composite draw,
  which is the same cut in the new coordinates. It is exercised by the existing backdrop
  goldens and by nothing new.
- **A layer's footprint is its contents' union**, so a group holding one small thing in a
  large clip is cheap, and a group holding two things far apart claims the box between
  them. Splitting that would need per-member ordering, which is exactly what the planner
  exists not to do.
