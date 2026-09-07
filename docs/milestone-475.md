# Milestone 475 — The bar that hugged its own back button

Closes [#12](https://github.com/KalybosPro/frus/issues/12).

`NavigationBar` asked for `Dimension::Auto`. In a row that means *hug your children*, so a
bar handed no width by its parent came out the width of its back button — and `paint`,
which centres the title in the box it is given because that is the only thing it can do,
put the title **underneath the arrow**.

## Why it took 296 milestones to see

Every screen in the demo happens to give the bar a width. A `Scaffold` puts it in a
column, a column stretches its children across, and the bug is invisible for as long as
nobody puts one anywhere else. It showed the first time the widget was rendered **on its
own**, which is what a golden does — milestone 296 drew the first one and the picture had
the title on top of the button.

That is the useful part of this bug, and it is worth writing down rather than fixing
quietly: a widget that is only ever seen inside one arrangement has never been asked what
it is. The demo is not a test of the widget; it is a test of the demo.

## The fix, and why it is a percentage

```rust
width: Dimension::Percent(1.0),
```

A chrome that spans the head of a screen has no other sensible answer to *how wide would
you like to be*, and an app bar gives the same one.

Not `flex_grow`, which was the other candidate and is wrong: grow acts along the parent's
**main** axis, and this widget's ordinary home is a column, where growing would make the
bar taller rather than wider. A percentage names the axis it means.

## What the test had to get right

A percentage of an undecided width is undecided too, so the reproduction needs a parent
whose own width is definite and whose child sizing is content-based — a row of a known
width. Give the root row `Auto` instead and the whole chain shrink-wraps, the percentage
resolves to nothing, and the test passes for the wrong reason while the bug is still
there. It was written that way first, and the number it printed said so.

The test asserts two things, because they are two different failures:

- the title is **centred in the frame** — the property the bar is for;
- and it is **clear of the back button** — which is what the bug actually looked like.

Verified by reverting the one-line fix and watching exactly that test fail on exactly the
first assertion, and nothing else in 1 903.

## The picture

`nav_bar_no_width`, in the file where the bug was first seen: a bar whose parent is a row
and gives it nothing, with the arrow on the left and "Settings" in the middle of the
frame. No existing golden moved — everywhere the bar was already handed a width, a
hundred per cent of that width is the same width.
