# Milestone 9 — Vertical scrolling + clipping

Adds clipping and a scrollable area.

## What ships

- **Per-primitive clipping**: each `Primitive` carries a `clip: Rect`;
  `Scene::set_clip` sets the current clip. Rectangles are clipped in the
  **fragment shader** (rejection outside the clip); text through glyphon's
  `TextArea.bounds`. Reusable everywhere (menus, cards, viewports).
- **`Scroll`**: a vertically scrolling container (fixed-size viewport). Its
  content is laid out **at free height** (`Layout::compute_unbounded_height`),
  then clipped to the viewport and translated by the offset.
- **Scroll offset**: runtime state, **keyed by `WidgetId`**, updated by the wheel
  and clamped to `[0, content − viewport]`.
- **Recursive driver**: `build_ui` carries a `(translation, clip)` context;
  `Ui::scroll_hit(point)` returns the scrollable area and its maximum offset.

## Architecture

```
build_ui walks the tree with a { translation, clip } context:
  - normal widget: paints at (rect + translation), current clip
  - Scroll (a leaf of the main layout):
        sub-layout of the content at free height
        translation += (0, −offset) ; clip = viewport
        records (id, viewport, max_height) for the wheel
Scene: every primitive carries its clip → GPU (shader / glyphon bounds)
```

A `Scroll`'s content is **excluded from the main layout pass** (the `Scroll` is a
leaf there) and laid out in a dedicated pass at free height, which stops
`flex-shrink` from crushing content taller than the viewport.

## Decisions

- **Per-primitive clipping** (shader + text bounds) rather than
  `set_scissor_rect`: compatible with our single-batch drawing and with text.
- **Runtime offset keyed by identity** (like focus) rather than in the
  application state: it is not business data.
- **A sub-layout at free height** to obtain the content's natural height.

## Demo

A **scrolling list** of items (taller than its viewport): the wheel scrolls it,
the items are **clipped** at the edges; the button adds items to the list.

## Tests

- `frus-core`: `set_clip` attaches the right clip to the primitives.
- `frus-gpu`: offscreen rendering — a rect whose `clip` excludes the centre
  leaves the centre showing the background (the shader clips).
- `frus-widgets`: a `Scroll`'s content is translated by the offset (expected y)
  and its clip = the viewport; max offset = content − viewport.

## Limits (next milestones)

- **Vertical only**, no visible scrollbar, no inertia.
- **Rectangular** clip (no rounded clipping).
- The content is painted in full and then clipped on the GPU (no culling of
  off-screen items).
