# Milestone 495 — A layer that moves, and a box that grows its share

Answers part of [#30](https://github.com/KalybosPro/frus/issues/30).

`AnimatedPositioned` and `AnimatedFractionallySizedBox`, and one mechanism under both.

## Two of the eleven, and why these two

The issue lists eleven implicit animations and asks for two or three at a time. Milestone
477 took `AnimatedScale`, `AnimatedRotation` and `AnimatedPadding`; milestone 488 took
`AnimatedAlign` and `AnimatedSlide`. These two leave `AnimatedDefaultTextStyle` of the
straightforward remainder, and the three the issue already separates.

They are taken together because they are **one mechanism**, the way a scale and a turn
were in 477 and an anchor and a slide were in 488. What both animate is a bundle of
**optional** numbers that a node hands to whatever lays it out — six for a stack layer
(four edges and two extents), two for a share of the parent — and both raise the same
question, which nothing animated here had raised before: *what does it mean to animate to
or from a number that is not there?*

## An unset slot is not a number, and does not travel

`None` on an edge is not nought on that edge. It is a layer **not pinned on that side at
all** — sized by itself and placed by the stack's alignment — and the two are different
arrangements rather than two values of one quantity. Interpolating between them would put
the box, for the length of the movement, in a state that means neither.

So the rule, written once in the runtime and shared by both families:

- a slot arriving takes hold **at once**;
- a slot leaving lets go **at once**;
- only a slot set at **both** ends travels.

The reference reaches the same rule from the other end: its tween for such a value is
dropped the moment the value is null, and constructed beginning *at* the target when it
appears, so neither direction moves. Reading its `_constructTweens` is worth it — the
whole rule is four lines and an `else`.

The consequence a caller has to know is that **a movement needs both ends to say where
they are**. A panel that slides away goes to `top: -56.0`, not to no top at all.

## One timeline for the bundle

The six pins share one clock, as a scale and a turn do. A layer changing two of its edges
arrives on both at the same moment rather than on two clocks that happen to agree — the
same reasoning as 477's, and the reason the runtime holds one `elapsed` per bundle rather
than one per slot. A change that lands mid-flight restarts the clock **from where the
values currently are**, so the movement stays continuous and only its timing is redone.

## Where each is consumed, and why they are not the same place

- **Pins** are read where the stack lays its layers out. A pin is a request to a parent of
  one particular kind, not a property of the box, and there is exactly one place that
  reads them. Both the size and the place come from the one value, so a layer cannot be
  measured against one frame's pins and placed against another's.
- **Fractions** are injected **at layout**, in `effective_style`, beside the animated size
  and the animated padding. A share of the parent is a claim on room, and a claim on room
  is settled before anything is drawn.

A fraction is injected as a `Percent` and not a `Length`, which is the whole reason to
reach for this rather than an animated size: neither end of the movement knows what it
comes to, and a window resized while it is running is answered by the layout rather than
by a number that was right when it was written. An axis whose factor is unset goes back to
**following its content**, not to nought — the same rule again, in the one place where
getting it wrong would collapse a box to nothing.

## `AnimatedPositioned` is a node of its own

`Positioned` is a transparent wrapper: it *is* its child, and the two are one node. That is
right for a pin, which says nothing about what the layer is. It is wrong for an animation,
because an animated value belongs to a node — a wrapper that fused with its child would
put the layer's timeline and the child's on the same one, and a layer that moved and
recoloured would have to do both on whichever duration was written last.

So this one has the child beneath it, which is the rule the whole of `animated.rs` is
built on: the animated node is one thing and its child is another, *so the per-node
animated values never collide*.

`AnimatedFractionallySizedBox` has no such problem — `FractionallySizedBox` was never
transparent — so the animation goes on the widget itself
(`FractionallySizedBox::animated`) and the named widget is a thin cover over it, the way
`AnimatedSlide` covers `FractionalTranslation::animated`.

## A bug found on the way: `Responsive` swallowed three hooks

477 predicted this and 488 repeated it: *a hook a forwarder does not list is a value the
runtime never finds*. `Responsive` — the size-class selector — forwarded twenty-odd hooks
and **not** `anim_offset` and **not** `positioned`.

- an `AnimatedAlign` or an `AnimatedSlide` behind a `Responsive` declared its target to
  nobody, so the runtime found nothing to drive and the child **jumped** between anchors;
- a `Positioned` behind one lost its pins entirely and became an ordinary layer filling
  the stack.

Both had been true since those widgets existed. Nothing noticed because nothing in the
framework or the demo had put either behind a selector — which is exactly how the first
one got shipped. Both are now forwarded, with `anim_pins` and `anim_fractions` beside
them, and each has a test that fails when its line is removed.

The lesson is not new and the cure has not been applied: the transparent wrapper has a
macro so that it *cannot* forget a hook, and `Responsive` is written by hand because its
`inner` is an `Option`. That is a difference of one `and_then`, and it has now cost three
silent bugs.

## Verification

Six unit tests. Each widget is pinned **mounted, mid-flight and at rest** as the issue
asks — a widget that appears already pinned or already at a fraction adopts it rather than
playing on the frame it was born — and the unset rule has one of its own, checking that
the edge that lets go and the edge that takes hold both do it at once.

Then the fourth test 477 recommended, twice, because it is the one that catches the bug
this framework has actually shipped: **does the value reach the screen?** One asserts the
stack places the layer at the interpolated pin and not the target; one asserts the child's
measured width is the interpolated share. Removing the single line in `ui.rs` that each
depends on leaves every value test green and fails exactly these two.

Three goldens, and they are **three frames of one movement** rather than its two ends,
because two ends are what a jump also produces. The middle one is the picture: the panel
neither up nor down, the rail neither a quarter nor full.

The pictures earned their place again. The rail in the first render came out **invisible**
— a fractional box sizes *itself* as a share of its parent and leaves its child to fill
it, and a child that stated neither a flex nor a height came out nothing at all. Every
value test passed either way, and the demo's own rail had the same mistake copied into it.

## In the demo

The guided tour. A rail under the panels fills as they go by — a fraction, since nobody
there knows how wide the footer comes out — and the way out slides off the top of the
panel once there is nothing left to skip, both ends pinning the same edge so that it is a
movement rather than a disappearance.
