# Jalon 3 — Declarative widget tree

Introduces the declarative layer: the interface is described with composable
**widgets**, automatically translated into layout and then into rendering.

## What ships

- **`Scene` moved into `frus-core`**: a pure display list (`Primitive`),
  independent of the GPU. `frus-gpu` consumes it (the `Painter` builds its
  instances from the primitives); `frus-widgets` produces it.
- **New crate `frus-widgets`**:
  - the `Widget` trait (layout style, children, painting),
  - the base widgets `Container` (decorated box) and `Flex` (row/column),
  - `build_scene(root, size)`: drives widget → layout → painting.
- **Demo** described in widgets (no manual layout calls left).

## Architecture

```
frus-core (pure Scene) ─┬─► frus-gpu     (renders the Scene)
                        └─► frus-widgets (produces the Scene)   [no GPU dependency]

Widget tree
   │ build_scene:
   │   1. Widget -> frus-layout nodes
   │   2. compute flexbox -> absolute rects
   │   3. each Widget paints its decoration
   ▼
 Scene -> frus-gpu -> screen
```

Pairing widget ↔ rectangle relies on an identical **prefix** walk on both sides
(the widget tree and `Layout::absolute_rects` produce the same order), so the
two can be zipped.

## Decisions

- **A retained model** (persistent tree) rather than immediate mode, but **with
  no reconciliation** at this stage (there is no state to diff yet). The
  abstraction is ready to receive it.
- **`Scene` in `frus-core`**: `frus-widgets` stays independent of the rendering
  backend (no wgpu dependency). Better modularity.
- **Widgets as trait objects** (`Box<dyn Widget>`): simple dynamic composition.

## API

```rust
let ui = Flex::column().padding(16.0).gap(12.0)
    .child(Container::new().height(56.0).color(green))
    .child(Flex::row().flex(1.0).gap(12.0)
        .child(Container::new().width(200.0).color(red))
        .child(Container::new().flex(1.0).color(blue)));

let scene = frus_widgets::build_scene(&ui, Size::new(w, h));
```

## Tests

- `frus-core`: `Scene::fill_rect` pushes the right primitive.
- `frus-widgets`: a `Flex::row` of `[Container(120px), Container(flex:1)]`
  (400×100, padding 10, gap 8) → `build_scene` produces 2 primitives at the
  expected absolute rects (reusing the flex computation validated in Jalon 2).

## Limits (next milestones)

- No **state** and no **events** yet (click, hover, input) — tree reconciliation
  will come with them.
- Few widgets (`Container`, `Flex`) and few style properties; no text.
