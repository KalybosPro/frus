# Jalon 30 — Window robustness

Fills in platform blind spots and lets the app **declare its window**.

## What changes

- **Zero-size guard**: if the window is minimised (`inner_size == 0`), rendering
  is **skipped** (avoiding GPU errors and pointless work) and `last_frame` is
  reset to `None` so that restoring does not cause a `dt` jump.
- **Occlusion**: on `WindowEvent::Occluded(true)` rendering is **suspended**; on
  `false` a frame is requested again. Saves the GPU while the window is hidden.
- **`ScaleFactorChanged`**: updates the scale **and reconfigures the surface** to
  the current physical size (not just the scale any more).
- **Minimum size**: the window is created with `min_inner_size` = 360×280 logical
  px (avoids an absurd UI).
- **Window DX**: a new `Application::window_size() -> Option<(f32, f32)>` (the
  initial **logical** size declared by the app).

## Trait

```rust
fn window_size(&self) -> Option<(f32, f32)> { None }   // initial logical size
```

The demo declares `Some((900.0, 680.0))`.

## Tests — being straight about it

These are **winit event guards**, not unit-testable without a real window.
Validation = **it compiles + the demo does not crash** + non-regression
(stopwatch, the 43 tests unchanged). No new unit test here, unlike the other
milestones — this is platform plumbing, and that is accepted.

## Limits (v1)

- No fine multi-screen handling or moving between monitors.
- `min_inner_size` is fixed (not configurable by the app).
