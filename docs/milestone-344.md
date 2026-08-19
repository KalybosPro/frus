# Milestone 344 — Text wraps

Milestone 343 left one difference in `Text` and named it: `softWrap` defaults to **on** in
the reference and was off here. A piece of prose put in a box narrower than itself was a
line that ran out of the box, and you had to say `.wrap()` to get a paragraph.

That is now the other way round, and the interesting part is not the flag.

## The flag was never the reason

The reason was that a `Text` **declared its own width**. A box that answers "this wide,
thanks" before anyone asks cannot be told to wrap: there is no width coming in for it to
wrap at. Turning the flag on without changing that would have changed nothing.

So a wrapping text is a **measured leaf** — free on both axes, its size computed from the
space offered, which is the only shape in which a box can be given a width and answer with
a height.

## A row does not squeeze; a column hands over

Making every text measured did change something, and the demo caught it within a minute: a
long task label took 1271 pixels of a 323-pixel row and pushed the delete button off the
card — the exact defect of milestones 333 and 334, back again.

A measured leaf reports its **narrowest useful width** as its minimum content, and for a
line of text that is the longest word. A row reads that as leave to squeeze the label down
to one word per line, and everything after it goes wherever it lands.

The reference draws a sharp line here, and it is not a flexbox line:

- **Along a row**, a flex leaves its inflexible children an **unbounded** main axis. They
  are never squeezed; they take their natural width, and if that does not fit, the row
  overflows and says so.
- **Across a column**, the same children are *handed* a width. That is where a paragraph
  is told how wide to be, and where it wraps.

Flexbox has one field for both — `min-width` — so the answer has to come from where both
the child and the parent's direction are in view. `Widget::main_axis_floor` is the child's
half: the width below which it will not be squeezed. The layout walk applies it only when
the parent runs horizontally, which is the parent's half.

With one exception, and it is the same exception `MainAxisSize::Max` needed two milestones
ago: a box with a **single child** — a padding, an alignment, a decorated box — is handing
a width down rather than dividing a line up, whichever way it nominally runs. The Kanban
screen's hint is a `Container(width, padding)` around a paragraph, and without the
exception it stopped wrapping and ran 73 pixels off the screen. The overflow survey caught
that one.

## Which texts are measured

Only those that need to be. A text that does not wrap is still a box of a known size and
still says so in its style — which is not an optimisation. It is what keeps it from being
folded, and it is why the floor above is needed for wrapping texts and not for these.

## What moved

Eighteen goldens, and every one of them by one to two pixels of vertical shift, with no
structural change anywhere. The cause: a measured height is `lines × line height`, where a
declared height was rounded up to the whole pixel. All eighteen were read side by side with
their predecessors.

Two demo tests failed first and both were right to: the task row (fixed by the floor) and
the board hint (fixed by the single-child exception). Nothing else in the framework's sixty
widgets noticed, which is what the split above is worth.

## Left

- **`Text::wrap()` is kept** although it is now the default. Saying it at the call site is
  not redundant when wrapping is the whole reason the widget is there.
- **A wrapping text inside a row of several children does not wrap.** That is the
  reference's answer and it surprises people there too; `Expanded` is how you ask for the
  other one.
- **The floor is measured per text**, on every layout of a row containing one. It is one
  cached measurement, but it is not free.
