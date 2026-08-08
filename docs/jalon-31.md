# Jalon 31 — Virtualised list (`List`)

The last of the roadmap (rich widgets). A `Scroll` lays out and paints **all** its
children every frame; for large lists (thousands of rows) that is O(N). The
virtualised `List` only builds, places and paints the **visible window**: the
per-frame cost is proportional to the visible items, not to the total.

## API

```rust
List::new(count, item_height, |index| row(index))
    .width(w).height(h)
```

As simple as a loop, but it scales to thousands of items.

## Mechanism

- The `List` is a **scrollable area**: it reuses all the scrolling machinery
  (runtime offset, wheel, inertia, bar). Content height = `count × item_height` →
  the scroll bound.
- The visible range is `[offset/h , (offset+viewport)/h]`; only those ~N items
  are **built on demand** (the `index → widget` closure), placed at
  `index×h − offset`, clipped to the viewport. Identity by **index**
  (`id.child(i)`).
- Trait hook `virtual_list(&self) -> Option<VirtualList<'_, Msg>>` (count,
  height, &factory); `build_ui` handles it as a special branch (like `Scroll`).

## Decisions & limits (accepted)

- **Fixed item height** (no variable measurement) — variable heights deferred.
- **Rendering through `render_item`** (and not the main `walk`): `walk` carries a
  lifetime `'a` in order to defer overlays; an item **built on the fly** cannot
  satisfy it. The consequence: an item is a **simple subtree** — no nested
  overlay, scroll or navigator, **no retained state per item** and no keyboard
  focus (we do not retain the state of an off-screen item). Clicking and hovering
  **visible** items: fine. That is the correct trade-off for virtualisation.
- Internal DX refactor: `full_status` / `draw_focus_ring` factored out and shared
  between the main rendering and `render_item`.

## Demo

A new **Log** screen (the "Log →" button): `List::new(5000, 44.0, …)` — 5000 rows
running smoothly, with only about a dozen built per frame.

## Tests

- `only_visible_items_are_built`: a counter proves that out of 5000 items, only
  ~5–8 are built (viewport 200 / item 40).
- `scroll_max_covers_full_content`: `max_y = count×h − viewport`
  (100×40−200 = 3800).
- `builds_a_scene`: non-empty rendering.
- 46 frus-widgets tests; demo and stopwatch did not regress.
