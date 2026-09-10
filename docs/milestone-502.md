# Milestone 502 — Two different children, both on the screen for a moment

Part of [#30](https://github.com/KalybosPro/frus/issues/30). Nine of eleven.

`AnimatedSwitcher`: when what it shows changes, the old child leaves as the new one
arrives — a fade by default, any explicit transition otherwise. Every other implicit
animation here moves a *number* on a widget that stays the same widget. This one is asked
for something the issue singled out as a different job: **two different children alive at
once**, and something deciding when the outgoing one is finished with.

## What is kept is the value, not the widget

The reference's switcher holds on to the old child widget, which works there because a
widget outlives the frame it was made in. Here nothing does. The view is a pure function
of the application's state, the tree is rebuilt from it, and a boxed widget can be neither
kept nor cloned — so the old child is simply not there to hold on to.

What the application gave to *make* the child is, though: plain data. So a switcher is
handed **a value and a way to build a child from one**,

```rust
AnimatedSwitcher::new(0.25, app.count, |n| text(n.to_string()))
```

and the runtime keeps the *values*, under the switcher's identity. A value changing is what
a switch is; equal values are the same child, which is the reference's key. The child on
its way out is **built again from its value on every frame it is on the screen** — a real
subtree, laid out and painted, not a picture of what it was.

That last point is the difference from what the shell already had. Since early on, a
widget that disappears is snapshotted and its primitives replayed, fading, over the frame.
That is not a switcher: it paints **over everything**, overlays included, in no one's
clip, on a fixed duration with no curve — and the test harness never captured it at all.

## Where it happens

A new value can only be noticed while the tree is being **built**: it is the only moment
both it and the one before it are at hand. The walks called `build_themed(theme)`, which
knows neither who the widget is nor what the frame remembers, so they now call
**`build_in(id, runtime, theme)`**, whose default is `build_themed`. Three walks call it —
the layout pass, the relayout fingerprint and `build_deferred` — and the transparent
wrapper, `Box` and `Responsive` forward it with the identity untouched. A test puts a
switcher behind a `Keyed` and fails when the wrapper's forward is taken out: built through
the older hook, the switcher never learns who it is and shows the newest value whole.

What a switcher builds carries each child's progress as a plain number, through the
caller's transition. So **while a switch is in flight the tree is built again each frame,
not only painted** — `Runtime::switching` tells the shell so, and it stops the frame the
switch settles. That is what a route transition already costs here, for the same reason;
it is one more instance of #53, not a new kind of cost.

## Each child has its own clock

Each child on the screen holds a linear `t`: up while it arrives, down while it leaves,
through the in-curve or the out-curve. A value arriving turns everything showing round
**from wherever it had got to**: a child cut off half-way in leaves from half-way, rather
than jumping to full and fading from there. A test switches twice in quick succession and
reads the middle child at one half; with the leaving child reset to whole on purpose, it
fails.

The first value is shown as it is — the mount rule every implicit animation here keeps —
and with motion turned down a switch is simply a swap, on the frame the value changes.

## The children on their way out are not the answer any more

They take **no input** and are **not announced**. A tap on a figure that is fading away
would be a tap on a state the application has left; a screen reader reading it out would
be reading that state. Both are wrappers the child wears whether it is arriving or leaving,
switched on as it starts to leave, so the subtree keeps its identity — and anything it
retains — across the switch. A test taps where only the old, wider child is: at rest it
answers, mid-switch nothing does, and only the new child's words are in the accessibility
tree.

The reference does neither by default. This is a deliberate difference.

## The box is the largest of the children — and the stack could not say so

The reference lays the children out in a centred stack, so the switcher is as big as the
largest of them while both show. A `Stack` here cannot do that: its layers are laid out
**apart from it**, so a stack with no size of its own is nothing — the same wall #52
describes, from another side.

So `frus-layout` gained **`Style::overlap`**: a grid of one cell, every child placed in it
(by `Layout::container`, the one place a parent and its children meet), the cell as big as
the largest child and stretched to a box the parent hands over, each child aligned in it
on both axes. A layout test pins both halves — hugging two children of 80×20 and 30×50 at
80×50 with each centred, and filling a 200-wide box — and a switcher test checks the narrow
new child is centred in the wide old one's box, then the box becomes its own.

The arrangement is the caller's too: `.layout(|children| …)` hands them over oldest first,
the arriving one last.

## A fourth hole in `Responsive`

`Responsive` is the size-class selector, written by hand where the transparent wrapper has
a macro, and that difference had cost three silent bugs (477, 488, 495). Adding `build_in`
to it showed a fourth: it never forwarded `build_themed` either. The walk asked it to
compose itself, the default did nothing, and `children` read straight through to a deferred
subtree nobody had built — an application bar is one. A test puts a `ThemeBuilder` behind
a selector and fails without the forward.

## Two things this does not do

- **The shell's own mount fade still applies to the arriving child.** Every newly mounted
  widget fades in over 0.12 s in the shell, so for its first 0.12 s the new child is under
  both that fade and the switcher's. It is short beside a switch, and the test harness does
  not model it; it is worth knowing when timing one by eye.
- **`AnimatedCrossFade` is not here.** Its point beside a switcher is that it *animates its
  size* from one child's to the other's, which needs both children measured — #52 again,
  with `AnimatedSize`.

## In the demo

The statistics card's big figure changes **in place** — a task ticked, another metric
picked — the old number shrinking away under the new one growing in, on an ease-out. It is
the reference's own canonical example, and the place an application would reach for it.

## Verification

- Twelve unit tests read off the scene: the first value as it is; a new value at its start,
  a quarter, half-way and at rest, the old child gone from the tree at the end; each
  direction on its own curve; a child cut off half-way leaving from half-way; an equal value
  not a switch; the rebuild requested while moving and not after; motion turned down; the
  leaving child taking no input and not announced; the box the largest while both show;
  behind a key; a switcher that goes being forgotten, so one that returns is a mount.
- The layout's `overlap`, hugging and filling.
- Rendered: half-way, the pixel is both colours, and at rest the new one alone; a golden of
  a count going from 3 to 4 caught half-way, with the fade and with a scale, looked at.
- Six mutations, each failing the test meant for it: `Responsive` not forwarding the build;
  the transparent wrapper not forwarding `build_in`; the leaving child keeping its input and
  semantics; a leaving child restarting from whole; a switcher's presence judged by identity
  alone (an identity is a position — the widget that replaced it has the same one, which is
  why there is a `switches()` hook); the in-curve ignored.
