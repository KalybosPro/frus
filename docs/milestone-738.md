# Milestone 738 — A rich paragraph in a selection

## Objective

What issue #24 had left. Milestones 573 to 578 made a `Text` selectable on its own and let a
`SelectionArea` select across texts, with a mouse, a finger, handles and a bar. A `RichText` — one
paragraph in several styles — took no part: a drag passed over it as if it were not there, Select
all left it out, and Copy never had its words (milestone 575, "Left"). It could not be made
selectable on its own either: `Text::selectable()` had no counterpart.

## What was done

- **A `RichText` inside a `SelectionArea` is one of its texts.** A selection can begin in it, end
  in it or pass through it, with a mouse, a double click, a finger's hold or a handle. The
  highlight covers the glyphs selected in whatever run they are in — a bold word, a larger run —
  and Copy takes its **plain text**: the runs' words, in order, one line for the paragraph, as for
  any other text of the area.
- **`RichText::selectable()`** makes one paragraph selectable on its own, as `Text::selectable()`
  makes a text: it takes focus and no typing, a drag, a double click or a long press selects, the
  caret keys move, Ctrl+C or the bar copies. **`RichText::selection_toolbar(build)`** is the
  application's say over its bar, as `Text`'s is.
- `frus-text` gained **`TextLayout::of_runs(runs, max_width, soft_wrap)`**: the caret, hit and
  selection geometry of a rich text, shaped as the renderer shapes it — the largest run's metrics
  as the base, each run's own size and line height on its glyphs. Its indices are characters of
  the concatenated runs. The plain-text layout and this one share the code that reads a shaped
  buffer's lines (`from_buffer`).

## Decisions

**The paragraph answers the hooks a `Text` answers, and the walk and the shell are unchanged.**
Milestone 575 made taking part a question of the widget (`selection_text`, `selection_hit`,
`selection_grip`, `region_toolbar`) and not of its type; a `RichText` only had to answer them. The
code a `Text` kept to itself for this — its bars' default list and cache, the highlight, the
handles — moved to crate functions in `text.rs` that both widgets call (`Selection`, `area_bar`,
`paint_highlight`, `paint_grips`), so the two cannot drift apart.

**Plain text, by character.** A selection's ends are character indices in the concatenation of
the runs, worked out once when the paragraph is built. A style boundary is not a boundary of the
selection, and a copy carries no styling: the clipboard here is text.

**The reader's setting the paragraph was painted under is remembered.** A hit is asked outside a
frame, where no font setting is in force; `Text` remembers what it resolved for the same reason.
A `RichText` remembers the scale its last paint ran under (at first, the one in force when it was
built) and lays its runs out under it.

**`Widget<Msg> for RichText` now asks `Msg: Clone + 'static`**, as `Text`'s has since milestone
573: the bars hold the application's messages. Every message boxed into a tree already satisfies
it.

## Tests

- `frus-text`: the plain layout's tests still pass over the shared line reader.
- `frus-widgets`, `richtext.rs`: the plain text is offered and only a selectable paragraph takes
  focus; a press past a 40-px letter lands where the runs are drawn, not where a 16-px layout would
  put it; a selection over the bold run is highlighted from where the plain run ends, as wide as
  the bold word, under the words; a selectable paragraph's own selection reads across runs, a
  double click takes a word inside a run, and it is highlighted only while focused; an area's two
  handles hang from it; a hit follows the font setting it was painted under.
- `frus-widgets`, `region.rs`: in a built frame the paragraph is registered between the two texts
  around it, a press finds it, and a selection through it paints one highlight per text, the
  paragraph's across all its runs and as tall as its largest.
- `frus-shell`, through the driver with a mouse: a drag from a text into the bold run, then
  Ctrl+C, puts the text's tail and `rich bo` on the clipboard, one line each; a drag beginning
  inside the large run copies its tail and the next text's head; a drag from the first text to the
  last copies the whole paragraph between them; a double click selects `bold` and paints its
  highlight; a paragraph made selectable alone copies `ch bold` across two runs with Ctrl+C.
- The driver's `texts()` now lists a rich paragraph's words as well (at its position: the
  primitive carries no measured box), and it gained `clipboard()`.
- The demo's About page test (milestone 575) now copies three lines: the rich line between the
  title and the paragraph is selected too, as plain text.
- The widgets example's `SelectionArea` section has a rich paragraph between its texts; the
  example builds (`cargo check`).
- fmt, clippy `-D warnings` (stable, 1.99.1, and `wasm32` for the widgets and the shell),
  `cargo test -p frus-widgets --lib --tests`, `-p frus-shell --lib`, `-p frus-text --lib` and the
  widgets' doc tests pass.

## Not done

- **Autoscroll** while a drag goes past the top or bottom of a scrolling area — the other
  leftover of #24 — is not done: a drag past the edge still selects to the nearest text on screen.
- A centred or end-aligned paragraph, or one cut by a line limit, is highlighted as if it were
  not — the limit a selectable `Text` has.
- Not run on a phone or in a browser. Goldens were not run: nothing paints differently unless a
  selection is held over a rich paragraph, which no golden does.
