# Milestone 734 — A field in a frame, and a grid given a height

## Objective

Two bugs found by building a page around frus: an editor beside a preview in a frame, the preview
being a frus application. Neither shows in an application alone in its window, which is why
nothing had found them.

1. **A text field in a frame kept the keyboard from the page around it.** Once a field in the
   frame had been pressed, nothing else on the page could be typed in, ever: a press on the page's
   own input gave it the focus for about 50 ms.
2. **A `GridView` with a height to spare came out uneven.** In an `Expanded` with room left, the
   first column of a grid of square tiles was several times wider than the others: 338 px against
   32 in the case the test keeps. A grid taller than its room looked right, so a screenshot of a
   long grid did not show it.

## What was done

### The focus (`web_editing.rs`)

While a field has the focus, the shell keeps the browser's focus on its editing element (J711),
and takes it back when it finds it "adrift": on the canvas, on the body, or on the field's node in
the accessibility tree. When the page around a frame takes the focus, the frame's
`document.activeElement` falls back to its **body**. From inside, that is indistinguishable from
the focus having gone nowhere, so the shell took it back on its next frame.

The shell now takes it back only while its own document has the focus (`document.hasFocus()`). A
press in the canvas still works: winit focuses the canvas on every press, which gives the document
the focus first, and the element is then adrift in the ordinary way.

### The rows (`frus-layout`)

Grid rows are `auto`, and CSS stretches `auto` tracks to fill a box of definite height
(`align-content: normal`). A tile with an aspect ratio then sized itself from the stretched row
instead of from its column. The grid now sets `align-content: start`: a row is what its cells say,
as the module's documentation already claimed, and what is left stays free at the end.

### The documentation (`grid.rs`)

`GridView::new` and `GridView::extent` are plain containers and do not scroll: what does not fit is
clipped. Only `GridView::builder` scrolls, being a list underneath. Each form's documentation
said what it was without saying that, and it cost a reader a bug report. The module says it now.

## Alternatives considered

- **Having the frame ask the page who has the focus** (`parent.document.activeElement`). The
  page around a frame can do this for itself. It only works on the same origin, and frus should not
  know it is framed; `document.hasFocus()` answers the question that matters from any origin.
- **Stretching tiles to their column explicitly** (`justify-self: stretch` in `set_tile_shape`).
  Not needed: the test passes without it once the rows stop stretching.

## Checks

- `a_grid_given_a_height_keeps_its_rows_and_its_columns`: fails without the fix with the 338 px
  wide tile, passes with it. A grid in a box of definite width and height reproduces it on its own.
- `scripts/web-frame-focus-check.py`, in a real browser: the demo in a frame beside an input of the
  page's. Without the fix the page's input never holds the focus (`'demo'` where `'own'` is wanted);
  with it, the focus goes to the page and back to the field, and each takes what is typed. Web-only
  code has no native test host, so this script is the test.

## Not done

The other forms of being adrift are unchanged. A press on the bare body of the **same** document
still sends the focus back to the field, deliberately (J711).
