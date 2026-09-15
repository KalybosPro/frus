# frus-gpu

frus's 2D renderer: it takes a `Scene` and turns it into pixels through
[`wgpu`](https://github.com/gfx-rs/wgpu) (Vulkan, Metal, DX12, WebGPU).

**Layer:** Foundations. It depends on no windowing library: the shell hands it a surface,
and it owns the device, the pipelines, batching, tessellation and the glyph atlas. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `Renderer`: created on a surface, `render(&scene)` once per frame.
- `render_offscreen`: the same pipeline into a texture read back as RGBA bytes, with no
  window. It returns `None` when there is no GPU adapter; golden tests are built on it.
- `draw_calls`: how many draw calls a scene will cost, to check that a screen batches.

## Example

```rust
use frus_gpu::{draw_calls, Color, Rect, Scene};

let mut scene = Scene::new();
for row in 0..10 {
    let y = row as f32 * 20.0;
    scene.fill_rect(Rect::new(0.0, y, 100.0, 16.0), Color::hex("#3B82F6"));
}
// Ten rectangles that do not overlap are one batch.
assert_eq!(draw_calls(&scene), 1);
```

## Part of frus

`frus-shell` drives the renderer for an application, and `frus-test` for tests. See the
[workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT or
Apache-2.0.
