# Milestone 592 — Built children go through the main walk

This is the first step of the plan milestone 587 proposed for #52. Before a measured container
can be the general tool, its child has to be painted by the same walk as the rest of the tree.

## What was wrong

Some widgets make their content during the frame:

- a list's rows;
- a paged view's pages;
- a `LayoutBuilder`'s content, and so an `OrientationBuilder`'s;
- a `ScrollOverlay`'s overlay.

Others measure their child in a layout of its own:

- `ConstraintsTransformBox` and `UnconstrainedBox`;
- `OverflowBox`.

All of them painted what they held through `render_item`, a reduced copy of the main walk.
Built content needed that copy: the main walk keeps references to the widgets it passes for
the whole frame, for the overlays painted at the end and the hero flights among other things,
and a row built on the fly did not live that long. The measured boxes used the copy even though
their child is part of the tree.

Milestone 587 expected a dropdown's menu not to show in an `UnconstrainedBox`. Run, the case
was worse: **the frame panicked**. The reduced walk visits every child in turn and takes one
layout box for each, where the main walk gives some children no box of their own. An open
dropdown in any of these containers read past the end of the boxes:

| where the open dropdown sat | before |
|---|---|
| at the root | its four options |
| in an `UnconstrainedBox` | panic |
| in a `LayoutBuilder` | panic |
| in a list's row | panic |

A **horizontal scroll view in a list's row**, a carousel in a feed, panicked the same way.

## What changed

- A frame keeps the widgets it builds in an **arena** that lives as long as the walk
  (`typed-arena`, a small crate with no dependencies of its own; this crate adds no `unsafe`).
  `Builder::keep` puts a built widget there and hands back a reference that lasts the frame.
- Every former caller of `render_item` now calls the main walk: the list's rows, the pages, the
  builder's content, the scroll overlay, the overflow box and the measured box. Built content
  goes through `keep` first.
- `render_item` is gone, and with it some hundred and thirty lines that had to be kept in step
  with the main walk by hand.

## What it brings besides the fix

Whatever the main walk knows, these children now get: overlays, transforms and opacity layers,
nested scroll areas, repaint boundaries, clips and the special branches. A row of a list is
now an ordinary subtree.

## Verification

- An open dropdown shows its four options in an `UnconstrainedBox`, in a `LayoutBuilder` and in
  a list's row, as it does at the root. On the old walk each of the three panicked; the test was
  run against it.
- A list of twenty rows, each a horizontal scroll view: the list and every visible row register
  scroll areas, and the rows scroll across. On the old walk this panicked; it was run against
  it too.
- **An overflow that had been hidden.** The overlay of a `ScrollOverlay` now goes through the
  main walk, which reports overflows. The collapsing header golden's collapsed bar showed the
  overflow band: its content was 45 px in a 44-px bar. The test's own header was too tall, and
  the reduced walk had never reported it. Its padding under the title went from 8 to 6 px, and
  the two collapsing header goldens were re-recorded and looked at. The other hundred goldens
  are unchanged.
- The demo's licence page, whose header has the same shape, was checked for the same fault
  and has none, open or collapsed. A test now says so, and fails with a 13-px overflow when
  the header's padding is made too big.
- On an Android phone, the demo's pages built by builders and panes (About, Stats, a metric's
  detail) draw as before.
- The whole library, the workspace and the goldens pass.
