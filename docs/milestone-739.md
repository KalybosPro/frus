# Milestone 739 — A selection that scrolls its area

## Objective

The last leftover of issue #24. Milestones 575 to 578 and 738 let a `SelectionArea` select across
its texts with a mouse, a finger, handles and a bar. Inside a scroll view taller than its box, a
drag that went past the top or the bottom edge only selected to the nearest text on screen: the
words below the fold could not be reached without letting go, scrolling and starting again (milestone 738, "Not done"; milestone 575, "Left"). And a selection whose beginning had
scrolled out of sight was lost: its ends were looked up among the texts in the frame, and the one
that had left was not among them.

## What was done

- **A selection dragged past the edge of the scrolling area it began in scrolls that area**, and
  keeps scrolling while the pointer stays there, moved or not. The speed grows with how far past
  the edge the pointer is — 30 px a second for each pixel, counted up to 20 px past, so at most
  600 px a second — and the area stops at the end of its content. Above the top it scrolls back,
  past the bottom it scrolls on, and either way along a sideways area.
- **The selection follows the content.** Each frame the area moves, the moving end goes to the
  words now nearest the pointer, as a move would take it: holding still below a long page selects
  down to its last line.
- **On a touch screen, a handle held past the edge does the same**, measured from where the
  handle points rather than from the fingertip; the bar comes back when the handle is let go.
- **The beginning is kept when it scrolls away.** The area remembers the texts its selection was
  worked out over, with their words, and a text that leaves the frame keeps its place among them:
  a selection that began on the first line of a scroll view still begins there once that line is
  out of sight, and Copy still copies it. (Before, a text was part of a selection only while it
  showed: the moment its first text scrolled away, the whole selection was gone.) A handle whose
  text is out of sight is not drawn and cannot be taken; the other one can.
- `frus-widgets` gained **`merge_text_stops`**, the pure function that puts the texts that left
  the frame back among the ones in it.

## Decisions

**The area is the one the drag began in.** The press records the innermost scrolling area under
it; when that one cannot move the way the pointer asks — a sideways strip under a pointer held
below a page — the areas around it are asked, outwards. A drag that began outside every scrolling
area scrolls nothing, as before. An area that refuses the user's own scrolling (its physics say
never, or nothing overflows) refuses this too.

**A point, not a box.** The drag target is the pointer (or the handle's line), with no size: the
overhang is how far past the edge it is. The law is the one carried rows already use (milestone
517), at a lower speed per pixel: what comes in is being read, not searched for a place to drop.

**The text that left keeps its place by its neighbours, not its position.** Painted order is
reading order (milestone 575), and a scroll removes texts from one end and adds them at the
other. A text that left goes back after the last text before it that is still in the frame, or
at the start when none is. If nothing of the old frame is left — a jump of a whole screen in one
frame, which this scrolling never makes — the old texts are taken to come first.

**Kept until the next selection.** The texts are remembered from one selection's beginning until
the next one begins, so Copy after the area has been scrolled by hand still copies what was
selected. A text that the application removed meanwhile is copied as it last read.

## Tests

- `frus-widgets`, `region.rs`: `merge_text_stops` keeps texts scrolled out above before the ones
  still there and those below after, puts a text gone from the middle back after its neighbour,
  doubles nothing and prefers the frame's words, reads downwards when nothing is in common; a
  selection whose first text scrolled away is empty over the frame alone and whole over the merged
  texts. A doc test shows the merge.
- `frus-shell`, the law: a point past the edge moves the content 30 px a second per pixel, capped
  at 20 px; inside, on the edge, or with nothing beyond, nothing.
- `frus-shell`, through the driver with a mouse, over a scroll view of thirty lines in a 200-px
  box: held 5 px past the bottom it scrolls 45 px in 0.3 s and 40 px past it scrolls 180 px; left
  there it reaches the end and stays; the selection then copies thirty lines, from the tail of the
  first — out of sight by then — to the last, which was out of sight when the drag began. Held
  above the top from the last line, it scrolls back to the start and selects up to the first line.
  A drag that stays inside scrolls nothing.
- `frus-shell`, with a finger: a word held, its end handle carried past the bottom and held there,
  scrolls the area to its end and the selection runs from the word to the last line.
- fmt, clippy `-D warnings`, `cargo test -p frus-widgets -p frus-shell -p frus-demo` pass.

## Not done

- **A virtualised `ListView`** still takes no part in a selection area, scrolling or not: its
  items are built for each frame and not kept in the tree, and the shell reads a text's words and
  hit-tests it through the tree, so a press on an item begins nothing. Found while writing this
  milestone's tests; a scroll view, or a list of texts in one, works. Making the list's items
  answer would need the list to keep what it last built, or the registry to carry each text's
  layout.
- Only a scrolling area **inside** the selection area, or the one it sits in, is scrolled; a
  selection area that holds several scrolling areas side by side scrolls the one the drag began
  in, as above.
- Words beyond the edge are reached as the area brings them in, not by where they would be: a
  pointer held past the bottom selects to the nearest text on screen in each frame, so the end
  catches up with the content as it scrolls.
- Not run on a phone or in a browser.
