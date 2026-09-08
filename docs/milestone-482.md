# Milestone 482 — A dialog's answers, when they stop fitting

Closes [#27](https://github.com/KalybosPro/frus/issues/27).

`AlertDialog` laid its actions out as a plain row. A row does not wrap, and `flex_shrink`
defaults to nought, so two buttons that stopped fitting neither folded nor squashed: they
kept their natural width and were drawn straight past the surface holding them.

The everyday way to see it is not a contrived label. It is a **reader's font size**: the
buttons grow with it, the dialog does not grow with them, and "Delete permanently" leaves
the screen. Which makes it an accessibility bug as much as a layout one.

## What the reproduction actually found

Three things were wrong, one behind the other, and only the first was the one in the issue.

1. **The row.** As described: a flex row that neither wraps nor shrinks.
2. **The surface.** A centred overlay is laid out at its **natural** size, on both axes, so
   a dialog at twice the ordinary font size did not overflow its surface — it *grew*, to
   715 px in a 400 px window, and spilled off both edges of the screen. With the surface
   free to grow, a bar of actions inside it would never once have folded: it was always
   offered exactly the width it asked for.
3. **The button.** Once the surface was clamped and the actions folded, the longest label
   was still wider than the dialog on its own — and a `Button` is a leaf that declares an
   exact width from its label and paints the label centred in whatever box it is given. Cap
   its box and the words came out of both ends of the pill.

Each of the three is load-bearing: removing any one of them fails
`long_actions_at_a_large_text_scale_stack_inside_the_dialog`, for three different reasons —
the buttons at `x = -143`, the surface 715 px wide, or the label straddling its own pill.

## `OverflowBar`

A widget of its own, as the issue asks, because a row of buttons that has to survive a
narrow screen is not unique to dialogs — the reference gives one to its banner, its bottom
sheet and its stepper too.

One line while the children fit on one line, a column when they do not. It takes the four
things the reference exposes: `spacing` between them on one line, `overflow_spacing`
between them stacked, `overflow_alignment` for the edge the column lines up on, and
`overflow_direction`, which matters because the conventions disagree with each other — the
confirming button goes **last** on a line and **first** in a column, so a bar that folded
without being told would move the destructive answer to where the thumb was reaching for
the safe one.

**All of them or none**, which is the whole difference from `Wrap`. A wrap fills each line
and moves what is left to the next: for chips that is exactly right, and for buttons it
reads as a mistake, the last one hanging alone under the others.

### How it knows, and what that costs

The available width is not known when the tree is built, so the arrangement cannot be
either. It is chosen during layout, on the `LayoutBuilder` mechanism: the bar is a measured
leaf, and the closure that composes it receives the box actually offered.

The **theme** arrives separately, and it has to, because a reader's font size lives in it
and the font size is the thing that makes a row stop fitting. So the closure builds an
arrangement that has the width but not the theme, and the arrangement answers
`style_themed` — measuring each child's natural width there, under the theme in force, and
returning a row or a column accordingly. The measurement is cached per arrangement: it is a
layout pass per child, and `style_themed` is asked more than once.

Three consequences, stated rather than discovered later:

- **The children are held by shared pointer.** The subtree is composed more than once a
  frame — once to measure, once to draw — which a `Vec<Box<dyn Widget>>` cannot survive.
  The same answer milestone 478 reached for a menu whose rows are widgets.
- **A deferred overlay declared inside a bar does not open.** Clicks, hover, focus and the
  accessibility tree all work, because the built subtree is walked by the same code an
  ordinary child is. A tooltip on a button in a bar does not, for the reason
  `LayoutBuilder` documents, and belongs around the bar instead.
- **Asked how wide it would like to be, a bar answers as a row.** An offer of nothing is an
  intrinsic question, and the honest answer to it is the width the children actually want.
  Answering "a column" would let a bar talk a dialog into being narrow and then complain
  that it was.

### No child is wider than the bar

Each child is capped at a hundred per cent of the bar's width — the constraint the
reference passes down in the same place. A ceiling of a hundred per cent leaves anything
that fits exactly as it was, and a percentage of an *indefinite* width is no ceiling at
all, so the intrinsic measurement above still comes back with the child's own width.

## The surface, clamped

A centred overlay whose natural size is wider than the room there is, is now laid out again
at that room. The room is the window less the content's **own margin**, which is how an
overlay says how far off the edges it wants to be held: a dialog's inset padding is exactly
that, and a clamp that ignored it would push the surface flat against both sides of the
screen.

This makes `DIALOG_INSET_PADDING` mean what its own test has always said it means — "held
off the window's edges by 40 across and 24 down". Before, a dialog wider than that simply
took the width it wanted and the margin was decoration.

**The height is deliberately left alone.** A dialog too tall for the screen wants its
content to scroll, which is a question of its own; squashing it here would only move the
spill inside the surface.

## The button

A box narrower than its label ellipsises it. A button asks for the width its words need and
nearly always gets it, so this is the rare case — a layout that had to squeeze one, which
is what a bar folding does to the longest answer at a large font size. What it must not do
is come apart.

## Not here

- **Pass-through parameters on the banner.** `MaterialBanner`'s actions are a bar now too —
  the same bug lived there, and the swap is one line — but it did not grow the three
  overrides `AlertDialog` did. Nobody has asked for them; the defaults are the reference's.
- **The stepper.** It has no row of buttons here to fix: its controls are markers.
- **Wrapping a squeezed label onto a second line**, which is what the reference's button
  does, because a `Button` here is a leaf that declares one exact height as well as one
  exact width. Ellipsising is the honest answer for a leaf; a button that reflows is a
  different widget.

## The pictures

`dialog_actions_on_one_line` and `dialog_actions_stacked`: the same window and the same
three lines of application code, with only the reader's font size different. One line at
the ordinary size; at twice it, the title wraps, the surface stays inside its inset, and
the two answers stack against the trailing edge.

There was no dialog golden at all before this — the issue's "the existing dialog goldens
must not move" turned out to be a requirement about an empty set — so these two now hold
both arrangements in place.
