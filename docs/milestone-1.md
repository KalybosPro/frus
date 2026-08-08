# Milestone 1 — Minimal 2D renderer (primitives)

Turns the Milestone 0 renderer (a hard-coded quad) into a **primitives API**: you
describe a [`Scene`] of coloured rectangles, and the GPU draws it.

## What ships

- **Drawing API** in `frus-gpu`: `Color`, `Rect`, `Scene::fill_rect`,
  `Renderer::render(&Scene)`.
- **Coordinate system** in logical pixels, origin top-left, Y downwards (the
  usual convention for UI and for CSS). Conversion to NDC happens in the shader
  through a `viewport` uniform.
- **Instanced rendering**: one unit quad repeated for each rectangle, with
  `{rect, color}` instance data in a buffer that grows on demand.
- **Alpha blending** enabled.
- **Headless render test**: renders a red rectangle into an offscreen texture
  and checks the centre pixel — automatic proof of rendering, with no window.

## Architecture

```
Scene (CPU: Vec<Instance{rect, color}>)
        │ queue.write_buffer
        ▼
 instance_buffer ─┐
 unit_quad (6 v)  ├─► Painter (pipeline) ─► shader:
 viewport uniform ┘        pos_px = rect.xy + quad * rect.wh
                            clip   = pixel_to_ndc(pos_px, viewport)
                            ▼
                         N rectangles on screen
```

How the `frus-gpu` modules are split:

| Module | Role |
|---|---|
| `color` | RGBA `Color` |
| `geometry` | `Rect` (logical pixels) |
| `scene` | `Scene` + `Instance` (GPU data) |
| `painter` | pipeline + buffers, **independent of any surface** (hence testable headless) |
| `renderer` | binds a surface (window) to the `Painter`, presents the frames |

Because the `Painter` is independent of the surface, the same render path serves
both the window and the offscreen test.

## Decisions

- **Instanced rendering** rather than tessellation: optimal for rectangles, and
  simple. Tessellation (complex shapes, curves) comes later.
- **Pixel coordinates** from the start: a prerequisite of the future layout
  engine.
- **Uniform buffer** for the viewport (portable) rather than push constants (not
  guaranteed downlevel or on the Web).
- **Instance buffer with growing capacity** (`next_power_of_two`): avoids
  reallocations when the scene is stable.

## Running / testing

```sh
# Demo window (3 rectangles):
bash scripts/wsl-run.sh

# Tests (including the offscreen rendering):
#   inside WSL, at the root:
cargo test
```

## Known limits (to be addressed later)

- No rounded corners, borders, or explicit z-order yet (order = insertion
  order).
- Colours passed through as-is (sRGB/linear handling deferred to a colour
  milestone).
- A single primitive type (`fill_rect`).
