# Milestone 485 — Two hundred pixels of nothing under a tab bar

Closes [#65](https://github.com/KalybosPro/frus/issues/65).

`TabBar::scrollable(true)` wraps its strip in a horizontal `SingleChildScrollView`. That
view's height defaulted to **200** — and 200 is the height of *a window onto something
taller*, which belongs to the axis that scrolls. A horizontal one does not scroll
vertically at all.

So a bar 48 pixels tall claimed 200, painted its hairline at 48, and everything under it
began 150 pixels into empty space. Filed during milestone 480 rather than fixed, because
the obvious repair does not work.

## Why the widget cannot answer it alone

Giving the still axis `Dimension::Auto` collapses the strip to nothing instead. A scroll
host lays its content out **against** the viewport on the axis that does not scroll — the
content of a horizontal strip is handed the viewport's height — so a viewport that said
`Auto` there would hand the content nothing and come back nothing tall. The circle closes
at zero.

Breaking it takes a measurement, and the measurement needs the runtime and the theme, which
only the layout pass has. So `Auto` on the still axis is now a **signal** rather than a
size: the widget says *this one is not mine to invent*, and the scroll branch of
`build_layout_scoped` lays the content out in a tree of its own and answers with what came
back. The same mechanism a `LayoutBuilder` uses, in the one place where both halves of the
question are to hand.

Two rules keep it narrow:

- **Only an axis that neither scrolls nor was given a size.** A caller's number is a
  caller's number, and an axis that scrolls has already answered.
- **Only the hugged axis is answered.** On the axis that scrolls, a viewport is precisely
  *not* as big as its content, and nought is what a leaf answered before this branch
  existed.

## The second rule cost a golden to learn

The first version answered both axes with the content's size, and the navigation drawer's
golden came back overflowing by 7.2 pixels with its last row gone.

The drawer's list is a **flexible** viewport: `flex_grow` with no height of its own, which
makes its flex basis its content-based size. That was nought while it was a leaf. Answering
with the content's real height made the basis the whole length of the list, the column's
bases exceeded it, and `flex_shrink` is 0 here — so nothing shrank and the column
overflowed by exactly the rounding of one row.

A viewport asked how big it would like to be, on the axis it scrolls, must answer *nothing*.
That is not a special case: it is what a window onto something taller means.

## What else it fixes

The same rule applies to a **vertical** area's width, which was `Auto` on a leaf with no
children — nought. In a column it was stretched to the parent and looked right; in a row it
came out nothing wide. It is its content's width now, and there is a test.

## The picture

`tabs_scrollable_with_panel`: six tabs in a bar too narrow for them, with a panel under it.
The picture is the whole bug — and nothing in the suite could have caught it, because
nothing rendered a scrollable bar with anything below it. Every scrollable test read the
strip's own geometry, which was right whatever box the scroll claimed.

The unit test pins the arithmetic: with the fix removed the panel is drawn at `y = 200`
against a bar 46 tall.
