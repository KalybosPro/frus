# Milestone 2 — Layout engine (flexbox via taffy)

Adds the layer that turns a **tree of styled nodes** into **positioned
rectangles**, ready for the renderer. We no longer position in absolute pixels:
we describe rules (flex, sizes, padding, gap) that adapt to the window.

## What ships

- **New crate `frus-core`**: fundamental shared types, with no logic and no
  dependencies (`Point`, `Size`, `Rect`, `Color`). `frus-gpu` re-exports them.
- **New crate `frus-layout`**: layout engine on top of
  [taffy](https://docs.rs/taffy), **hidden** behind a stable frus API.
  - `Style` (width/height, flex_grow, flex_direction, padding, gap),
  - `Layout<T>`: tree carrying a `T` per node (here a `Color`),
  - `absolute_rects()`: rectangles in **absolute coordinates**.
- **Demo** driven by layout: a column (bar + sidebar/main row), adapting to the
  window size.

## Architecture

```
        frus-core  (Point, Size, Rect, Color) — zero dependencies
        ╱        ╲
  frus-gpu       frus-layout (wraps taffy)
        ╲         ╱
          frus-shell  (layout -> Scene -> GPU)
```

The flow of a frame:

```
tree of nodes (Style + Color)
      │ taffy::compute_layout
      ▼
relative positions ──(accumulating offsets)──► ABSOLUTE rects
      │ Scene::fill_rect
      ▼
   frus-gpu ─► screen
```

## Decisions

- **taffy** for layout (mature flexbox/grid, used by Bevy and Zed) — reuse the
  ecosystem rather than rewrite it.
- **A shared `frus-core`**: avoids the `frus-layout → frus-gpu` coupling and the
  duplication of the `Rect` type.
- **A thin API** on top of taffy: a frus `Style` is translated into a
  `taffy::Style`. taffy stays a replaceable implementation detail.
- **Absolute coordinates** computed on the frus side (taffy gives relative
  ones): directly renderable and testable.

## Tests

- `frus-core`: construction and `to_array` for `Rect`.
- `frus-layout`: a flex row `[fixed 120px, grow:1]` inside 400×100 with padding
  10 / gap 8 → checks the absolute rects (`A = (10,10,120,80)`,
  `B = (138,10,252,80)`).

## Running

```sh
bash scripts/wsl-run.sh   # window: green bar + red sidebar + blue area
cargo test                # inside WSL
```

## Limits (to be addressed later)

- A flexbox subset only (no alignment or per-side margins yet).
- No widget tree on top: the demo builds the tree by hand. The widgets milestone
  will come and build on this layer.
