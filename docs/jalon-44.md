# Jalon 44 — Dynamic scale & size

Three additions on the **shell/platform** side so that responsiveness reacts to
size and density **live**.

## Batch A — User density / scale

`Application::density(&self) -> f32` (default `1.0`): an **application-level**
zoom factor applied on top of the system DPI scale. The shell computes
`total_scale = system_scale × density` and uses it everywhere:

- the **logical** size passed to `view` = physical / total_scale (the UI grows or
  tightens);
- the scene is scaled by the total at render time (crisp on HiDPI);
- the cursor, the wheel (PixelDelta) and the back gesture's width are divided by
  the total.

The app changes `density` through a message → the whole UI zooms (like browser
zoom), with no widget having to care.

## Batch B — Breakpoints driven by the real size

`Application::on_resize(&mut self, width, height)` (default no-op): the shell
tracks the current **logical** size and, on **every change** (window resize *or*
density change), calls `on_resize` **before** `view`. The app can then react to a
**tier change** in its logic (closing a drawer as it narrows, resetting a
selection…), not only in its rendering.

`SizeClass` is re-exported from `frus-shell` for use on the app side.

## Batch C — Smooth resizing

`Resized` and `ScaleFactorChanged` reconfigure the surface and request another
frame; `RedrawRequested` always rebuilds the `view` at the **live** logical size
and triggers `on_resize` on the slightest difference — so the responsive reflow
follows the drag with no latency and no stale surface. A density change (through
a message) also forces a redraw, hence the same path.

## Demo

**A− / A+** buttons in the header: they zoom the whole UI (density `0.8..=1.4`).
`on_resize` remembers the current tier (`size_class`), closes the Stats detail
when going Compact, and logs every tier change.

## Tests

- `frus-demo`: density clamped (`0.8..=1.4`, with a `0.0 → 1.0` guard);
  `on_resize` updates the tier and closes the detail in Compact.
- The winit wiring (cursor/scene scaling) is not unit-testable (as with the
  window milestone) — validated by compilation + the demo without regression.

## Limits (v1)

- No animated zoom transition (density changes instantly).
- `on_resize` is called along with the redraw (no dedicated out-of-frame event).
