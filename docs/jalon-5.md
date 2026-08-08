# Jalon 5 — Text

Adds text rendering: measurement (for layout) and GPU rasterisation (through a
glyph atlas). The UI can finally show labels that react to state.

## What ships

- **New crate `frus-text`**: measurement of a line of text through
  [`cosmic-text`](https://docs.rs/cosmic-text) (`measure(text, px) -> Size`).
  The `FontSystem` is initialised lazily and shared (behind a Mutex).
- **`frus-core`**: new primitive `Primitive::Text { position, text, size, color }`
  plus `Scene::text(...)`.
- **`frus-widgets`**: the `Text` widget (sizes itself by measurement, pushes a
  text primitive when painting).
- **`frus-gpu`**: text rendering through [`glyphon`](https://docs.rs/glyphon)
  (cosmic-text + atlas + wgpu pipeline). The `Painter` (rectangles) and the
  `TextPainter` (text) draw in the **same render pass**; the renderer separates
  `prepare` (uploads) from `draw` (recording into the pass).
- **Demo**: a button labelled "+ Add a square" and a "Squares: N" counter that
  updates on each click.

## Architecture

```
frus-core   : Primitive::Text (pure data)
frus-text   : measure(text, px) -> Size          [cosmic-text, global FontSystem]
frus-widgets: Text widget → style()=measure ; paint()=Scene::text
frus-gpu    : TextPainter (glyphon) renders the Primitive::Text in the render pass
```

## Decisions & simplifications (v1)

- **Reuse**: `glyphon` (rendering) + `cosmic-text` (measurement) rather than a
  hand-rolled text pipeline — fast and robust to get right. A hand-rolled
  pipeline (SwashCache → atlas → shader) stays possible later for total control.
- **A single line**, the default system font, left alignment.
- **Two separate `FontSystem`s** (measurement vs rendering): a known
  inefficiency, to be unified behind a shared font context.
- `prepare`/`draw` split in the renderer so that rectangles and text share a
  single render pass.

## Tests

- `frus-text`: `measure` > 0 for non-empty text, zero width when empty.
- `frus-widgets`: the `Text` widget emits the right `Primitive::Text`.
- `frus-gpu`: **offscreen rendering** — white "Hello" on a black background
  produces non-black pixels (automatic proof of rasterisation, with no window).
- `frus-shell`: `view` produces the right number of primitives (button
  background + labels + squares).

## Prerequisites (WSL)

A system font is required: `apt-get install -y fonts-dejavu-core fontconfig`.

## Limits (next milestones)

- No wrapping, alignment, or rich styles (bold, italic).
- Two `FontSystem`s to unify.
- Hover/pressed/focus visual states and the keyboard: still waiting on
  reconciliation.
