# Milestone 477 — Three of the fifteen that should move rather than jump

Part of [#30](https://github.com/KalybosPro/frus/issues/30) — three of the eleven implicit
animations it lists. The issue asks for two or three, not eight, and this is
three: `AnimatedScale`, `AnimatedRotation`, `AnimatedPadding`.

Without them, each of these is an application holding its own `Anim`, stepping it by hand,
and getting the curve slightly different from every other application.

## Two of the three are one mechanism

A scale and a turn are the same thing to the paint walk: both are numbers melted into a
single affine matrix and applied to a composited layer. So they are one animated quantity,
`TransformValues`, driven by one timeline — which also means a widget that scales *and*
turns arrives on both at the same moment rather than on two clocks that happen to agree.

`Transform::animated(duration, curve)` is where it lands. The widget declares where it is
going through `Widget::anim_transform`, and the runtime drives it there, on the machinery
every other implicit animation here already runs on. `AnimatedScale` and
`AnimatedRotation` are transparent wrappers over a `Transform` that was told to animate.

### The pivot does not animate, and that is a decision

`TransformValues` holds two scales and a turn. It does **not** hold the pivot, because a
pivot is a choice of origin rather than a quantity: interpolating it would slide a shape
across the screen while every number describing the shape stood still. A widget that wants
its pivot elsewhere has changed what it is doing, not how much of it.

### The default is the identity, not zeroes

`TransformValues::default()` is scale `1, 1` and turn `0`. Deriving `Default` would have
given zeroes, and a scale of nought is a shape with no area — so a transform mounting at
the default would flash its whole subtree out of existence for one frame. Written down
because the derive is the obvious thing to reach for and it is wrong here.

### Scale interpolates linearly

Halfway between 1× and 4× is 2.5× here, not 2×. That is what the reference's own
`Tween<double>` gives and what a caller reading the two numbers expects; a geometric mean
is defensible in isolation and would disagree with every other implicit animation in the
framework.

## `AnimatedPadding` was already built, and had a hole under it

The runtime has animated padding — `PaddingAnim`, `advance_paddings`, `anim_padding` — and
`Container::animated_padding` has had it since long before this. So the widget is four
lines.

It did not work. `forward_to_container!`, the macro every widget in `animated.rs` is built
on, forwards twenty-odd hooks to the inner `Container` and **`anim_padding` was not one of
them**. Nothing had noticed because no wrapper had ever set a padding: the runtime walked
the tree, looked through the wrapper, found no target, and drove nothing. The first widget
to need it is the one that found it.

Unlike the other two, this one is **layout**: the interpolated value is injected while the
tree is measured, so everything beside and below really moves aside. A paint-time offset
would slide the child over its neighbours instead of making room, which is the whole
difference between padding animating and a transform animating.

## The test that matters is the one about the paint

Each of the three is pinned at mount, mid-flight and at rest — the issue's own criterion —
and mount is the one worth stating: a widget that appears already scaled **adopts** its
target rather than growing into it from nothing. Without that rule every implicit animation
in the framework would play once on the frame it was born, and a page would breathe when it
opened.

But those three tests only ask whether the runtime computes the right number. A fourth asks
whether the **paint reads it**, which is a different question — and the difference is not
theoretical here: disabling the one line in `ui.rs` that consults the tween leaves all
three runtime tests green and only the fourth red. A tween the walk never asks for is a
number that moves correctly and changes nothing on the screen.

## In the demo

The drawer menu's active entry makes room for itself. An inset that jumps when the
selection moves reads as a relayout; one that slides reads as the selection moving, which
is what actually happened.

## Not here

- **`AnimatedSwitcher` and `AnimatedCrossFade`** — the issue says these are a different job
  and should not share a pull request with these, and it is right: both keep two different
  children alive at once and something has to decide when the outgoing one is finished.
- **`AnimatedSize`** — it animates to a size it has to *measure*, and a widget cannot
  measure its children in this framework. The same wall #27 runs into.
- **`AnimatedSlide`** — its offset is in units of the child's own box, so it needs the
  child's size at paint time. Reachable, but it is a different quantity from the three
  numbers above and belongs with whoever adds `AnimatedAlign` and `AnimatedPositioned`.
- **`AnimatedDefaultTextStyle` and `AnimatedFractionallySizedBox`** — the remaining two of
  the first eight, left for the next pull request the issue asks for.
