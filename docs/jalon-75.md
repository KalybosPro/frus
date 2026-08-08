# Jalon 75 — Text decorations (underline, strikethrough, highlight)

## Analysis

`TextStyle` (milestone 60) covered size/weight/italic/colour but no
**decoration**: there was no way to strike out a completed task or underline a
link. The established vocabulary is a `TextDecoration` (underline / overline /
line-through, combinable) plus a `decorationColor`; this is the last brick of the
§5 spec on the text-attribute side (`letter_spacing`/`line_height` remain, out of
scope).

The key constraint: neither cosmic-text nor glyphon draws decorations — a text
engine shapes glyphs, and the lines are the business of the rasteriser above it
(as in any toolkit where the paragraph paints them itself).

## Architecture

- **`frus-core`**: `TextDecoration { underline, overline, strikethrough }` (Copy,
  combinable through `combine`, with `UNDERLINE`/`STRIKETHROUGH`/… consts).
  `TextStyle` gains `decoration` + `decoration_color: Option<Color>` (`None` =
  the text's colour). `TextSpan` cascades both through its partial `Overrides`;
  `TextRun` and `Primitive::Text` carry them to the GPU.
- **`frus-gpu`**: the decoration quads are computed **from the laid-out lines**
  (`buffer.layout_runs()`), so they are exact even with wrapping (`max_width`)
  and mixed runs. `TextPainter::prepare_frame` returns
  `DecorationQuad { rect, color, clip }`; the rectangle `Painter` draws them in
  its pass (under the glyphs — with the same colour: indistinguishable).
  - Plain text: one line per `layout_run`, from the first to the last glyph
    advance.
  - Rich text: each span gets `Attrs::metadata(run index)`; consecutive glyphs
    from the same run form one decorated segment — so the decoration really is
    **per-span**, not per-line.
  - Offsets from the baseline (`line_y`) as a fraction of the run's size:
    underline +0.12 em, strikethrough −0.28 em (≈ mid x-height), overline
    −0.90 em; thickness `max(1, size/14)`.

## Decisions

- **No effect on measurement**: decorations are excluded from
  `measure_hash`/`measure_key` (like colours) — recolouring or striking out a
  paragraph does not invalidate the relayout cache.
- `merge`: the decoration is a *type* attribute (`over`'s wins wholesale), while
  its colour *inherits* like the text colour. Inside a `TextSpan`, an explicit
  `decoration(NONE)` cancels the inheritance (`Some(NONE)` ≠ absent).
- The Renderer's order is reversed: text is prepared **first** (it produces the
  quads), rectangles after. No change to the passes.
- A `decoration_style` (dotted, wavy): not taken up — there is no consumer, and
  it would need a dedicated pipeline.

## Implementation

- `frus-core/text_style.rs`: the type + builders (`.underline()`,
  `.strikethrough()`, `.decoration()`, `.decoration_color()`) on `TextStyle`
  **and** `TextSpan`; the fields on `TextRun`.
- `frus-core/scene.rs`: `Primitive::Text` gains both fields;
  `text_styled`/`text_wrapped` take them from the style; `push_faded` fades the
  decoration colour, `scaled` carries it (the thickness derives from the
  already-scaled size).
- `frus-gpu/text.rs`: `DecorationQuad`, `push_line_quads`, the computation
  through `layout_runs` (+ per-span `metadata` for rich text); `painter.rs`
  accepts the quads alongside the scene; `renderer.rs` reorders the preparation.
- Widgets: `Text::underline()/strikethrough()/decoration*()`; `RichText` carries
  the fields (the exit fade applied to the decoration colour).
- Demo: a completed task = a greyed **and struck-out** label; "portable" in the
  rich tagline underlined.

## Tests (253 → 256)

- `decorations_combine_and_cascade` (core): combination, inheritance within
  spans, explicit cancellation, `merge` semantics.
- `underline_lights_more_pixels_than_plain_text` (gpu, readback): the same text,
  underlined, lights strictly more pixels — end-to-end proof (quad computation +
  the rectangle pass).
- `rich_text_strikethrough_is_per_run` (gpu, readback): the second run's
  strikethrough adds pixels — proof of the metadata → per-run segments path.

## Known limits

- Decorations are drawn **under** the glyphs (the rectangle pass): with a
  different decoration colour, a strikethrough passes behind the glyph's ink,
  where the conventional behaviour paints it in front. With the same colour:
  indistinguishable.
- The offsets are approximated (fractions of an em) rather than read from the
  font's metrics (`post.underlinePosition`) — sufficient for DejaVu; to be
  refined if exotic fonts arrive.
