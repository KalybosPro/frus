# Milestone 489 — The explicit transitions, and the twelve that were not missing

Advances [#32](https://github.com/KalybosPro/frus/issues/32).

`FadeTransition`, `SlideTransition` and `ScaleTransition`, in a new `transitions` module —
and a mapping for the rest of the reference's fifteen, which is most of the content of this
milestone.

## Why there are fifteen there and three here

In the reference a widget that animates has to **listen**. An `Animation<double>` is an
object; each transition subscribes to one and rebuilds its subtree when it ticks. That is
what those classes are for, and it is why there are fifteen of them: every property that can
be animated needs a class that knows how to rebuild for it.

There is no listening here, and no retained tree. The view is rebuilt every frame and an
animation is a number in the `Runtime` — so a widget that takes the number *is* the explicit
form, and this framework's widgets already take numbers. `Opacity` takes an opacity;
`Transform` takes a scale; `FractionalTranslation` takes a fraction. Nothing had to be built
for them to be driven by an application's own clock; the family was already there under
other names.

So the honest report on `transitions.dart` is a map rather than a list of gaps:

| the reference's | here |
|---|---|
| `FadeTransition`, `SlideTransition`, `ScaleTransition` | this module |
| `RotationTransition` | `Transform::rotate` on a value, or `AnimatedRotation` |
| `PositionedTransition`, `RelativePositionedTransition` | `Positioned` with interpolated pins |
| `AlignTransition` | `Container::alignment` |
| `MatrixTransition` | `Transform`'s composed matrix |
| `AnimatedBuilder`, `ListenableBuilder`, `AnimatedWidget` | **nothing** — the subtree is rebuilt every frame anyway |
| `SliverFadeTransition` | waits on slivers, which this framework has not got |
| `SizeTransition` | needs to measure a child ([#52](https://github.com/KalybosPro/frus/issues/52)) |
| `DecoratedBoxTransition`, `DefaultTextStyleTransition` | real work, and their own milestone |

The three in the middle row of that table are the ones a **transition between two states of
a screen** is made of, in the reference too, which is why they are the three that got names.

## What a name buys when the wrapper is thin

The issue anticipated this and answered it: *named, shared, tested, and identical
everywhere*. Three things came out of writing them:

- **`SlideTransition::from_edge`** is the one that earns its keep. *Coming in from the left*
  is a negative offset that shrinks to nought as the progress grows — a sign and a
  subtraction that every application writes once and gets wrong once. It is one function
  here, with a test that pins each edge's direction.
- **The progress convention is stated in one place**: `0` is not started, `1` is arrived,
  and past `1` is an overshoot that is deliberately *not* clamped, because that is what a
  spring does at the end of its travel and clamping it would flatten the one moment the
  spring exists for.
- **No transition tweens.** An explicit transition is already where the caller says it is;
  giving one a duration of its own would set it chasing a value the application is itself
  stepping, and the two clocks would fight. Stated in the code where somebody would
  otherwise add one.

`eased` came with them: `curve.transform(t)` under the name of the thing a caller is looking
for, so an explicit transition and an implicit one given the same curve are on the same
shape.

## In the demo

Every screen is now told **how far in it is** — the same number the navigator is driven by,
read from the other end. That is the reference's `ModalRoute.of(context).animation`, and the
point is where it comes from: the application owns the controller, hands it to the navigator
*and* to the screen, and the two cannot disagree.

The task screen uses it: its words slide up and fade in behind the avatar as the screen
arrives, and run backwards when it leaves — the progress is `1 - progress` for the screen on
its way out, which is a line in `build_view` and not a second animation.

**A wrapper is a box, and the demo's own test said so.** Putting the two lines inside a
transition changed what they were measured against: the body column centres its children, so
a block with no width of its own comes out at the width of its own content, and the title —
which wraps — measured at its natural length, one long line, and ran under the label below
it. That is the failure milestone 334 first caught on a device, and the test written then
caught this one in the same place, at the same assertion, a hundred and fifty-five
milestones later. The words are
given the full width explicitly now, and the comment says why.

**The avatar is deliberately not in it.** It is a shared element, and the framework is
already flying it from the row the screen was opened from; a transition on top would be two
hands on the same object.

## The picture belongs with the settled widgets

`explicit_transitions` is in `goldens.rs` and not in `motion.rs`, and that is the family's
whole character: an explicit transition takes the number itself, so its picture **is** a
function of its arguments. There is no runtime to prime and no frame loop to step. The
gestures in `motion.rs` are there precisely because they are not functions of their
arguments; these are.

Three rows at 35 %: a fade, a slide straddling the edge of the slot it is heading for, and a
scale small in the middle of the box it will grow to fill.
