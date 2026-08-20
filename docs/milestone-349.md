# Milestone 349 — Nothing is squeezed unless it says so

Flexbox's default is that every child of a row absorbs its share of a deficit. The
reference's is that it absorbs none: an inflexible child of a row or a column is laid out
with an **unbounded** main axis, keeps the size it asked for, and if the line does not fit,
the line overflows — visibly, with a striped band and, since milestone 348, a sentence
saying by how much.

This framework had flexbox's. That is how a 40 px delete button came to be laid out at
13 px, then off the card entirely and out of the hit registry, so that a task could not be
deleted — and it took milestones 333 and 334 and a report from a device to find, because
nothing said anything. The whole point of the reference's rule is that a layout which does
not fit **says so** rather than quietly producing a smaller, wronger one.

So `flex_shrink` now defaults to `0.0`, and `shrink(1.0)` is how a box asks for the other
behaviour.

## The exception, which is the same exception three times

A box with **one** child is not dividing a line up. It is handing its own constraints down
— a padding, an alignment, a decorated box — and the reference bounds a lone child by the
constraints it was given. So the walk grants a lone child the right to give way
(`Layout::allow_shrink`), and that is the third time this exact exception has been needed:

- milestone 342, the fill request: a run fills a parent that runs the same way **only**
  when it is the only child;
- milestone 344, the main-axis floor: a child may refuse to be squeezed along a row of
  **several**, and not in a box holding it alone;
- and now this.

Three different questions, one answer, which is a sign the answer is the real one: *a box
with one child passes constraints down; a box with several divides a line.*

Growing does not count as asking for something else. A lone child that fills its parent is
still bounded by it — a box that both grows and refuses to give way can only end up bigger
than the thing holding it, which is what happened to the task screen's column before the
guard was relaxed: it filled its parent, refused to shrink, and drew its title 611 px wide
in a 376 px window.

## What it found

Everything the old default was hiding. Twelve tests moved; three of them were **pinning
the defect**, and the other nine were layouts one or two pixels short that flexbox had
been quietly paying for.

- **The task card's footer** needed 365 px in a 323 px card on a phone. It wraps now,
  which is the reference's answer too: the line that does not fit becomes two lines.
- **The journal's header** was 25 px over. The label expands and ellipsises instead of
  being followed by a spacer — same pushing when there is room, and it is the one that
  gives way when there is not.
- **The journal's list height** was computed by hand as `height - 152` where 156 was
  right. Four pixels, paid for by the list quietly giving way, for as long as that line
  has existed.
- **A golden of four text blocks** in a 120 px box needed 121.
- **A navigation rail** of three destinations is 198 px tall and was being squeezed into
  190.
- **A `TwoPane`** asks for the *whole* of its parent's height and was sharing that parent
  with two other things; it is expanded now, which is what it meant.

Not one of these was a new defect. Every one of them was already wrong and already
invisible.

## What is left as it was

The Settings screen still overflows by 4.5 px at a phone's width, unchanged. What is new
is a sharper measurement: its tab set is **380 px wide whatever the viewport** — the same
380 at 411 px and at 260 — so this is a hard minimum inside the Controls tab and not a
proportion of anything. The panel column sits at 340 and the two numbers differ by the
tab set's inset and the panel's padding. Two earlier diagnoses (`Tabs` shrink-wrapping,
milestone 335; disproved in 345) were wrong; this is the third measurement and the first
that constrains where to look.

## Verified on the device

`XMJNW19B23011768`, a release build. The task card's footer wraps onto two lines and
nothing is clipped; the journal header pushes `Switch` to the right edge with no cut; the
settings panel fits; the Kanban board and the drawer are unchanged.

The device showed two things that are **not** this milestone's, both recorded for their
own:

- A group-opacity **layer from the page below shows through the page above** — the home
  screen's translucent square is painted over the Kanban board, and thin slivers of a
  swipe background appear at the left edge. Layers are composited after the content
  batches, so a layer belonging to a covered page lands on top of the page covering it.
  Nothing in the compositor was touched here.
- **A virtualised list's rows hug their text** instead of filling the list's width. The
  reference gives a list's children *tight* cross-axis constraints; ours leaves them free,
  so a row with a background paints a chip rather than a row.

## Left

- **`no_shrink()` is now the default said out loud.** Kept, because a layout that depends
  on it reads better for saying so, but it is no longer doing anything.
- **`Flexible` is still spelled `Expanded::loose()`.** The reference has two names —
  `Flexible(fit:)` and `Expanded` — and an application ported from it will type the one
  that does not exist here.
- **`shrink()` is on `Flex` and `Container` only**, while ten other widgets carry
  `flex()`. It matters less than it did — the default is what those widgets wanted — but
  the asymmetry is still there.
