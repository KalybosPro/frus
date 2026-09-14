# Milestone 516 — A bar whose actions ran past its edge

Found by the demo's own overflow test, the day the demo's stopwatch came out of its header:
one action fewer in the bar, and on a desktop the bar kept one more action inline than there
was room for — 13 px past its right edge, on every screen.

## Why

`AppBar` keeps as many labelled actions on the line as fit and folds the rest into its `⋯`
menu. How many fit is decided **before** the row is laid out, because the answer decides which
children the row has. So it was decided on arithmetic, and the arithmetic described a bar that
was not the one being built:

- **Each action was an estimate.** The label's width and 20 px either side of it — a constant
  whose comment said it "must follow `button::PAD_X`", which no longer existed. A button takes
  24 px either side, is never narrower than 64, and rounds its width up to the pixel. Short
  labels lost the most: an `A+` was counted at 60 and drawn at 68.
- **The `⋯` was the same estimate**, so the one button a folding bar always has was short too.
- **The row had a gap the budget did not.** Between the title and the actions the row puts a
  spring, and so two gaps; the budget counted one. A centred title's second spring was not
  counted at all. And without a leading, the budget still charged the leading's spacing — too
  much, where the rest was too little, so the errors did not even lean the same way.

Each slip is a few pixels. A sweep across widths found them adding up to as much as **34 px**
past the edge, in bars whose labels are nothing unusual — the demo's own.

## The fix

**The button is made in one place.** `action_button(label, message, size)` is the outlined
button an action is drawn as; the fold asks that button for its width, through the same
`style_themed` the layout reads, and the row then draws that same button. The `⋯` goes through
it too. There is no second description of a button left to drift from the first — which is
what the old constant was, and what its comment was trying to prevent by asking a human to
remember.

**The row is counted as the row is built**: the leading, the spacer that widens its gap to
`title_spacing`, a spring before a centred title, the title and the spring after it, each
joined to the next by a gap; the join to the actions is each action's own, and a bar with no
actions still owes it. The actions' padding, when there is one, is charged as well.

Nothing changes for a bar whose actions already fitted with room to spare: the fold only
moves where the old arithmetic said an action fitted and it did not.

## Verification

**Two tests.** A sweep builds the demo's own eleven labels into a bar at every seventh width
from 240 to 1600 px and asks the layout whether anything ran past its box. It does so with an
icon-button leading as the demo has, with a labelled leading whose width is declared exactly,
with none, with a centred title and with padded actions. On the old arithmetic the first
version of it found an overflow in 262 of the 585 bars it built, up to 34 px wide. A second
test sweeps a long title with no actions at all.

Each configuration starts at the narrowest bar it can be. Below that, with every action already
folded, the leading, the title's 64 px floor and the `⋯` are wider than the bar on their own:
a dump of the layout at those widths showed the title at its floor and nothing left to give,
so an overflow there is the content's and not the fold's.

A first version of the sweep used only the icon-button leading, and three mutations survived
it. An icon button is charged the whole 56 px leading slot and drawn narrower, and those 8 px
of slack hid exactly one forgotten gap. The labelled leading, whose width the budget knows to
the pixel, is what closes it.

**Mutations**, seven, each failing a test: the actions estimated instead of measured; the `⋯`
estimated; the gap after the title, the spacer, a centred title's spring and the actions'
padding each left out of the budget; and a bar with no actions owing no gap.

**The demo's overflow test** — every screen, at a phone's width and a desktop's — passes
again, as do the widget, shell and demo suites, clippy across the workspace, and the goldens,
none of which moved.
