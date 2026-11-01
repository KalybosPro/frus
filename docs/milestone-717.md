# Milestone 717 — A list keeps the reader's place

## Objective

Milestone 716 gave frus `ListView::builder`, a lazy list of rows of any height, and left one
thing undone: when a row **above** the window changes height, the rows below it move. A
picture finishing loading in a post half scrolled past pushes the post being read down the
screen; scrolling back up through rows never measured, each one longer or shorter than the
guess it was counted at, makes the rows in view jump at every one. The reference corrects the
scroll offset when that happens, so that what the reader is looking at stays where it is.

## What was done

- **The window is measured before it is placed** (`ui.rs`, the variable-extent path). The
  items the window reaches are laid out first; each one that **starts above the window's top
  edge** and turns out longer or shorter than it was counted adds the difference to a
  correction. The offset moves by that correction, and the items are then placed from it — so
  the rows in view stay where they were and the row above grows or shrinks upwards, out of
  sight. At the very top of the list nothing starts above the edge, and a row that grows
  pushes the others down, as it should. The items measured in the first pass are kept and
  placed in the second, so nothing is built twice.
- **The correction reaches the region's offset** through the runtime, which the layout only
  reads: `Runtime::scroll_corrections` collects it during the walk, and
  `Runtime::apply_scroll_corrections`, called by the shell at the top of the next frame (the
  window's and the test driver's), adds it to the offset, to where a glide is heading, and to
  a fling under way — a `ScrollBallistic` now carries a `shift` its positions are offset by, so
  a fling glides on from the corrected place instead of jumping back. Each correction is
  applied once.
- **The test driver honours `host::scroll_to`.** The window's frame applied scroll requests;
  the driver's never did, so a test could not scroll a list to where the bug shows. It now
  takes them at the same point, and retries one whose region is not laid out yet.

## Tests

- `tester.rs`: a 100-row feed is scrolled to the middle of row 3, then row 3 grows by 100
  pixels; row 6, on screen, stays where it was. The test fails without the correction (row 6
  is pushed off the screen) — checked by running it against the layout with the correction
  disabled.
- The tester's other eight tests, and the frus-shell and frus-widgets suites.

## Not done

- **A row that changes height while entirely above the window** is not noticed until it is
  laid out again, which is when it scrolls back into view; the correction happens then, in
  the frame it comes back. Measuring rows nobody can see would cost what laziness saves.
- **`scroll_to` an index** in a list of rows of any height: done in milestone 719.
