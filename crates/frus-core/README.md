# frus-core

The vocabulary every other frus crate speaks: geometry, colour, paths, text styles,
animation and the `Scene` a frame is drawn from.

**Layer:** Foundations, the bottom one. It has a single small dependency (`bytemuck`) and
knows nothing about windows, GPUs or platforms. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `Scene` and `Primitive`: the flat display list. Widgets push primitives into it and the
  renderer draws it, and nothing else crosses from one to the other.
- `Color`, `Rect`, `Size`, `Point`, `Insets`: the value types every layer passes around.
- `AnimationController`, `Tween`, `Curve`, `SpringSimulation`: time and physics, shared
  by every animated widget.

## Example

```rust
use frus_core::{Color, Rect, Scene};

let mut scene = Scene::new();
scene.fill_rect(Rect::new(0.0, 0.0, 120.0, 40.0), Color::hex("#3B82F6"));
assert_eq!(scene.len(), 1);
```

## Part of frus

Applications rarely name this crate: the `frus` facade and `frus-widgets` re-export the
types above. See the [workspace README](https://github.com/KalybosPro/frus#readme).
Licensed under MIT or Apache-2.0.
