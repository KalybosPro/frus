# Milestone 501 — A decoration and a text style, driven by the caller's number

Part of [#32](https://github.com/KalybosPro/frus/issues/32).

`DecoratedBoxTransition` and `DefaultTextStyleTransition`: the two rows of the explicit
animation library that milestone 489 left as "real work, and their own milestone", because
neither is a thin wrapper over one value. Each interpolates a whole — a decoration's fill,
gradient, border, corners and shadow; a text style's size, weight, colour, face — and the
real work is `BoxDecoration::lerp` in `frus-core`, one rule per part.

## One `Option`, two meanings, two rules

Milestone 498 taught `TextStyle` to interpolate, and decided that **a colour set at one end
only does not travel**: it holds still. That was right, because a text style that names no
colour is *asking the theme for one* — `on_surface`, a real and visible colour — and fading
it out would make words vanish half-way through a movement that was about their size.

A decoration that names no fill is saying something else entirely: **nothing is painted**.
So here the same `Option<Color>` gets the opposite rule — the colour on one side only
**fades** — and that is not an inconsistency but the two meanings of `None` taken seriously.
Both rules are written beside each other in the two `lerp`s, so the next reader does not
have to rediscover why they differ.

## And fading is where this repository's most repeated bug was waiting

The obvious way to fade a colour out is to interpolate it towards `Color::TRANSPARENT`. But
`TRANSPARENT` is transparent **black**: half-way from red to it is a half-opaque *dark* red,
so a fill fading out goes dark on the way. The fade here keeps the colour's hue and moves
only its alpha, which is what the reference does with a colour one side lacks.

It is tested twice, because a colour slip is caught by the pixel it produces rather than by
the number handed to the renderer. The unit test reads the colour; a rendered test fades a
red fill half out over white and samples the pixel: **the red channel stays at 255** and the
other two come up together — pink, not dark red. With the obvious fade put back on purpose,
the same pixel renders **(204, 187, 187)** — a greyed, darker red, measured rather than
reasoned, and failing that test along with the two that read the numbers.

## Each part by its own rule

- **Colours mix** where both sides have one, in the space colours are stated in — as every
  animated colour in this framework already does.
- **A part on one side only arrives or leaves; it does not come from nowhere.** A fill
  fades in its own hue. A border **thickens from nought in its own colour**, rather than
  arriving as a colour on its way from nowhere. A shadow **grows out from under the box as
  it fades in** — scaling its geometry alone would leave a hard-edged block of shadow colour
  under a box whose own fill may be fading too.
- **A flat fill is a gradient whose two ends agree.** Here a gradient *starts* at the fill
  colour — `paint_into` draws nothing for a gradient with no fill — so flat to graded
  spreads the far end out of the fill instead of laying a gradient over it, and the two
  always arrive and leave together.
- **Corners** travel one by one — through `BorderRadius::lerp`, which is now also what an
  animated container's corners go through. There was a private copy of the same arithmetic
  in the runtime; an animated container and a decoration transition cannot round
  differently now.

`t` is clamped, unlike a slide's: past its ends a colour has nowhere to go, and a
decoration whose colours stopped at the end while its corners kept going would be two
clocks. At `0` and `1` the answer is the end itself, exactly.

## The border does not push the content

`Container` insets its child by its border's width, so that a line never eats the content.
For a transition that is the wrong model: a border thickening over a transition would push
the content about **on every frame of it**. So `DecoratedBoxTransition` is its own node, the
reference's `DecoratedBox` — it paints behind (or, through the walk's foreground hook, over)
a child it does not inset. A test grows a border from nothing to eight pixels and checks the
child never moved; with the inset put back on purpose, it fails.

## Why one is a node and the other is not

`DecoratedBoxTransition` is a node because a decoration is something painted, and something
painted has a box.

`DefaultTextStyleTransition` is a **transparent wrapper** — which its implicit twin,
milestone 498's `AnimatedDefaultTextStyle`, could not be. An implicit animation keeps a
timeline in the runtime, a timeline belongs to a node, and two of those nested would share
one. An explicit transition keeps nothing: the caller's number is the whole of its state. So
two nested fuse into one node and hand down both of what they say — the outer asking the
inner, as a nested theme does. A test puts a colour on the outside and a size on the inside
and reads both off the painted words.

## In the demo

The task screen's title already slid and faded in on the route's own progress (milestone
489). It now also **grows into place** on the same number — 20 to 24 — so the three arrive
together. It names no size of its own for that reason, and at rest it is the 24 it always
was, so nothing a golden or the title-wrapping test watches has moved.

This is not the demo replacing something it hand-rolled, which is what #32 asked for: the
demo never interpolated a style by hand. It is the demo using the transition where it was
already driving an explicit one, which is the honest version of that.

## What is left of #32

- **`SizeTransition`** animates a size it has to measure, and a widget cannot measure its
  children here — still waiting on #52.
- **`SliverFadeTransition`** waits on slivers.

Everything else in the reference's file is either here or maps onto something that is
(milestone 489's table).

## Verification

- `BoxDecoration::lerp`: the ends exact (and past them, and at NaN); a fill fading out keeps
  its hue; one fading in arrives in its own colour; flat spreads into graded; a border
  thickens in its own colour; a shadow grows and fades in; corners one by one.
- The transitions, read off the scene at the start, half-way and the end; a fading fill
  painted in its own colour; a thickening border that does not move the child; in front
  covers the child and behind is the default; the words drawn at the style in flight; two
  nested handing down both; a text that names its size keeping it.
- The same fade, **rendered**, over white: full red, pink not dark.
- A golden of both at three moments, looked at.
- Two mutations — the fade towards transparent black, and the border insetting the child —
  each failing the tests it should.
