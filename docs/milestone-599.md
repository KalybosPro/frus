# Milestone 599 — `SizedBox::expand` fills the way the rest does

Milestone 598 found this while writing tests for `LimitedBox`. `SizedBox::expand` in a column
inside a scroll was **0 px wide**.

## Why

`expand` gave the box a width and a height of 100 % of its parent. A percentage is of the
parent's size, and a column inside a scroll is as wide as its content, which is what it is
waiting to learn from the box. There was nothing to take 100 % of. The same box directly on a
page worked, because a page has a size of its own.

Every other widget that takes the room, such as `Aligned`, `Center`, a list or a field, asks for
it instead (`Widget::fill_axes`, milestone 590). The request travels up, and each container
answers it: grow along its own axis, or stretch across it. A column inside a scroll stretches
across to the scroll's width when something in it asks.

## The change

`SizedBox::expand` now asks for the room on both axes and has no size of its own. It is sized as
`Aligned` is:

- It fills all the room its parent has to give: a page, a card, the width of a column even
  inside a scroll.
- Along a row's or a column's own direction, where the line is shared, it is as long as its
  child. Wrap it in `Expanded` for the rest of the line.

It also **hands its child** the room it took, as the reference's does with tight constraints.
The box is one cell, stretched both ways. Asking for the room alone was not enough: the box was
200 px wide and a plain coloured container inside it still hugged nothing.

The reference's `SizedBox.expand` asks for infinity and fails where the room has no end. This
one takes what it can get.

`width_fraction` and `height_fraction` are still percentages, which is what they say.

## Verification

- In a column inside a 200-px scroll, a plain coloured container in it is 200 px wide. It was 0.
- Alone on a page, it is the page.
- In a column with a size, it is the column, as before.
- In a row beside a label, it is as wide as its 30-px child and as tall as the row, after the
  label.
- Mutations, each failing a test:
  - no request for the room;
  - the old percentages;
  - the child not stretched.

  The old percentages survived at first: the test's child was a container with a flex factor,
  which filled the box by other means. The test now uses a plain one.
