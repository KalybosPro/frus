# Jalon 62 — `TextSpan`: rich text, from the styled tree to the GPU

A continuation of the typography thread (§5). Milestone 60 laid down `TextStyle`
and the rendering of weight and italic; this one brings **rich text**: several
styles mixed within one paragraph, shaped as a single piece (one baseline).

## `TextSpan`: the styled tree with cascading inheritance (frus-core)

A `TextSpan` = a fragment of text + **partial** overrides + children. The key
point: a `.bold()` child **inherits** its parent's size and colour — which
merging complete `TextStyle`s cannot express (it would overwrite the size). The
partial overrides live in an internal type (`Overrides`, every field optional)
composed through the builders: `.bold()`, `.weight()`, `.italic()`, `.size()`,
`.color()`, `.style(TextStyle)` (a complete override).

`flatten(base)` flattens the tree into **resolved runs** `(text, TextStyle)`, in
reading order, cascading down from the paragraph's base style. Nodes with no text
of their own ("style groups") produce no run.

## The pipeline, end to end

- **`TextRun`** (frus-core): a run ready to render — text plus **resolved**
  size/weight/italic/colour.
- **`Primitive::RichText { position, runs, … }`** + `Scene::rich_text`; `scaled`
  scales the runs' sizes, `push_faded` fades their colours.
- **`frus-gpu`**: a single cosmic-text buffer per paragraph, through
  **`set_rich_text`** — each run carries its `Attrs` (weight, italic,
  **per-span metrics** for mixed sizes, **per-span colour** which glyphon applies
  per glyph). The base metrics = the largest run.
- **`frus-text::measure_runs`**: measurement of shaped rich text (the width of
  the longest line, the real height `line_top + line_height` — mixed sizes
  count).

## `RichText`: the paragraph widget (frus-widgets)

`RichText::new(span).base_style(theme.text.body_large)` — the base style is the
root of the cascade; inherited colours are resolved against the theme **at paint
time** (and modulated by the opacity). The natural size is measured by
`measure_runs` (no automatic wrapping for now, as with `Text`).

Demo: the tagline on the About screen mixes bold, italic and a coloured segment
(`no GC` in `theme.primary`) within a single sentence.

## Validation

- **End-to-end GPU proof**: `renders_rich_text_to_non_background_pixels` —
  offscreen rendering + readback of a paragraph with mixed runs (40 px regular +
  24 px bold) on a real wgpu device; the readback harness is factored out and
  shared with the plain-text test.
- The cascade: 3 frus-core tests (inheritance of unspecified attributes, a deep
  cascade, group nodes with no run) + a doctest.
- Rich measurement: `rich_runs_measure_mixed_styles` (wider with a 24 px bold
  segment; the height driven by the largest run; empty → zero).
- The widget: runs resolved against the theme, inherited bold, an explicit
  colour; the layout height follows the largest run.
- **226 tests** in total, all green (core 52, widgets 138, gpu 5, text 4, demo
  15, shell 7, layout 3); a warning-free build; the demo did not panic.

## What's next (§5, text)

- **`TextLayout`** on cosmic-text: `hit_test`/`caret_rect`/`selection_rects` and
  min/max intrinsics → the brick for migrating `TextInput` and, eventually, the
  wrapping paragraph (measurement under constraint, taffy measure closures).
- Decorations (underline/strikethrough), `letter_spacing`/`line_height` in
  `TextStyle`.
