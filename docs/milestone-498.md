# Milestone 498 — A style handed down, moving

Eight of eleven for [#30](https://github.com/KalybosPro/frus/issues/30):
`AnimatedDefaultTextStyle`, the last of that issue's straightforward remainder — and the
only one of the eleven whose value never reaches the node it is written on.

## The one that is not like the others

Every animated value in this framework is read from the runtime **by whoever consumes it**:
a colour by the paint, a size by `effective_style` at layout, pins by the stack that places
the layer. Each has one consumer, and finding it is most of the work.

This one's consumer is the **theme swap** — the step where a walk asks a widget whether it
replaces the theme its subtree inherits. That is how a subtree hands a text style down here,
and it is the same mechanism `DefaultTextStyle::around` uses; the animation only makes the
handed-down style move.

The swap happens **four times over a frame**, and that is the milestone:

| where | why it makes the swap |
|---|---|
| `build_layout_scoped` | a theme reaches sizes, so a subtree lays out under its own |
| `hash_node` (relayout) | the fingerprint must be taken against the same theme, or the cache lies |
| `walk_node` (paint) | a widget must paint against what it was measured with |
| `build_deferred` | a builder inside the subtree composes against the theme that reaches it first |

Four copies of one rule are four chances to drift, and this codebase has the receipt:
`Responsive` is written by hand where the transparent wrapper has a macro, and that
difference has now cost **three** silent bugs (milestones 477, 488, 495). So the swap is one
function, `ui::scoped_theme`, and the four sites call it.

### Which is why `build_deferred` grew a runtime

It had no way to reach one, and the honest options were to give it one or to write down a
limitation. The limitation is not small: a `ThemeBuilder`'s subtree is built **once**, into a
`OnceCell`, by whichever walk arrives first — and `build_deferred` always arrives first. A
builder inside a moving style would therefore compose against the target for the whole of
the movement, while the layout and the paint around it used the value in flight.

So it takes a runtime. It is one frame behind there, because it runs before the frame
advances its animations, and one frame behind is the whole tree agreeing with itself rather
than one builder disagreeing with everything around it.

## What moves, what holds, what swaps

`TextStyle::lerp`, and it is three rules rather than one, because a text style is three
kinds of thing wearing one name:

- **Sizes and ratios travel.** `size`, `height`. The obvious case, and the only one where
  "halfway between" needs no argument.
- **A field only one end states does not travel.** It holds the one value it has, from the
  first frame to the last. `None` means *this style does not say* — the question passes
  further up the cascade — and there is no number between "24 pixels" and "ask somebody
  else". Interpolating towards a value nobody stated would mean interpolating towards
  whatever the theme happens to answer, which makes the movement depend on the theme and is
  what neither end asked for. **A movement needs both ends to say where they are**, which is
  milestone 495's rule for an unset pin reached again from a different direction. The
  reference lands on it from a third: it takes the stated value for *both* of its own ends.
- **Faces and lines swap at the halfway point.** `italic`, `decoration`, `family`. There is
  no half-italic face and no two-thirds of an underline, so the change happens once, in the
  middle, where it is least noticeable. The reference's rule.

`weight` is the near miss and gets its own answer: it *steps*. A weight is drawn with a
face and there are four faces, so interpolating the number would ask for a 437 nothing can
draw. What is interpolated is the **position in the list**, rounded — the reference's method
over its nine, not a lookalike, and the one that stays right if a fifth face is ever added
between two of these. A tie goes to the heavier face, so a run of words thickening and one
thinning pass through the same face at the same moment instead of missing by a step.

### The colours diverge from the reference, deliberately

There, a null colour means *nothing was painted*, so fading towards it is right, and its
own interpolation scales the alpha. Here `None` means *the widget resolves it against the
theme at paint* — `on_surface`, a perfectly visible colour. Fading it out would make a run
of words **disappear halfway through a movement that was only ever about its size**. So a
colour follows the second rule above and does not travel unless both ends state one.

## Only the type moves

Alignment, wrapping, overflow and the line count take effect the moment they change, which
is what the reference says of its own in as many words. They are arrangements rather than
quantities — there is nothing between wrapping and not — and the same reasoning that keeps
an unset pin from travelling keeps these from being tweened at all.

## A node of its own

Like `AnimatedPositioned` (495), and unlike the transparent `Themed` it is built on the same
idea as. Two of these nested — an outer moving the colour, an inner the size — is a thing
the cascade positively invites, and fused into one node they would be two timelines on one
identity, the second overwriting the first every frame.

## Verification

**Mounted, mid-flight, at rest**, as the issue asks of each of these. Then the fourth test
milestone 477 recommended, which has earned its place every time it has been asked and earns
it hardest here: *does the value reach the screen?* Because this family's consumer is so far
from its node, a correct tween and a correct target can sit either side of a subtree that
reads neither. Taking the merge out of `scoped_theme` leaves every value test green and
fails exactly one — the one that reads the size off the painted run of words.

**And the cache's half**, which is new. The same tree at two points of one movement must not
share a layout fingerprint. A font size reaches layout, so a fingerprint blind to the style
in force would answer every frame of the movement with the first frame's geometry: the words
growing inside a box that did not, and the layout snapping into place at the end. Reverting
`hash_node` alone to the plain `theme_override` fails that test and nothing else.

**And the demo**, where the failure would be silence. A task's label had its colour and its
line through stated on the text itself; they are handed down now, so that both can move. A
field a caller states on a particular run of words wins over what its subtree says, so
getting this wrong makes a ticked task look exactly like an active one — on a screen where
every row still lays out, still paints and still passes every count. The row is asked what
it actually drew, with the active row beside it as the control.

**The picture**: three frames of one movement, and the middle one is the argument. The
colour is between the two and the strikethrough is not between anything — it appears at the
halfway point, once, because a line cannot be half-drawn. That frame is taken at three
fifths rather than at a half on purpose: the swap happens exactly at the middle, so a golden
taken there would turn on whether two floats added up to a fraction under or over.

## Still open on #30

`AnimatedSwitcher`, `AnimatedCrossFade`, `AnimatedSize` — the three the issue separates, and
which are untouched for the reasons it gives. The first two animate between two *different
children*, so both are alive at once and something has to decide when the outgoing one is
finished with; the third animates to a size it has to **measure**, and a widget cannot
measure its children in this framework ([#52](https://github.com/KalybosPro/frus/issues/52)).
