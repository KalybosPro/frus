# Jalon 64 — Measuring under constraints (taffy closures) + wrapping paragraph

The last gap in the layout protocol (§1 of the brief): "intrinsic sizes routed to
**taffy's measure closure** — for text and custom-painted content". Until now a
`Text` measured its natural size and **froze** it into its style: long text
overflowed or was clipped, never wrapped to the parent's width.

## Measured leaves (frus-layout)

- **`MeasureFn`** = `Box<dyn Fn(Option<f32>, Option<f32>) -> Size>`: it receives
  the maximum width and height (`None` = free) and returns the content's size.
- **`Layout::measured_leaf(style, data, measure)`** — the closure is retained per
  node (`HashMap<NodeId, MeasureFn>`), **without touching the tree's context
  type**.
- Both computation paths go through **`compute_layout_with_measure`**; the
  constraint translation gives the **intrinsics for free**: `min-content` → width
  `Some(0)` (the longest word), `max-content` → `None` (the natural size).

## The paragraph: `Text::wrap()`

- `style()` → free dimensions; **`measure()`** → an owned closure (the content is
  cloned) over `frus_text::measure_wrapped` (cosmic-text wrapping under a
  constrained width); `paint()` → **`Scene::text_wrapped`**.
- **`Primitive::Text` carries `max_width: Option<f32>`**: GPU rendering wraps at
  the **same width as the layout** (before, glyphon wrapped at the surface width —
  never reached). `scaled` scales the wrapping width for DPI.
- New `Widget::measure` / `Widget::measure_key` hooks, delegated by
  `Box<dyn Widget>`, `Keyed` and `Responsive`. Text **without** `.wrap()` is
  strictly unchanged (frozen dimensions, no closure).

## The relayout cache's trap — fixed

The cache (milestone 55) only fingerprints **style + structure**. But a measured
leaf's content affects the geometry **without going through the style**: two
different paragraphs with the same styles would have shared a signature — and the
cache would have served an **old layout**. Hence **`measure_key()`** (a
fingerprint of the content: text + size + weight + italic), mixed into the
relayout signature. The documented contract: `measure()` and `measure_key()` are
`Some` together.

The `wrapped_text_wraps_in_layout_and_invalidates_the_cache` test pins exactly
that scenario: the same tree, the same runtime (a warm cache), different content
→ the clickable follower **moves** (recomputation), and the paragraph wraps its
lines within the column (pushing the follower down).

## Validation

- `frus-layout` **4 tests** (+1: a measured leaf wrapped to the offered width, 3
  lines expected); `frus-text` **10** (+1: wrapping bounded in width, height
  growing); `frus-widgets` **140** (+2: the paragraph's measure/key plus the
  end-to-end layout + cache test). **236 tests** in total, all green.
- Demo: the About screen gains a paragraph wrapped to the card's width. A
  warning-free build; the demo did not panic.

## What's next

- A wrapping `RichText` (the same mechanics, measuring over runs).
- The rest of §5 on the colour side: consolidating `ColorScheme`,
  `content_padding` → taffy (measured leaves open the way to measurements with
  padding).
