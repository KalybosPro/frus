title: The batch planner is O(n²)
labels: help wanted, performance, rendering

`frus_gpu::batch::plan` gives every primitive a level from what it covers, and it finds
that level by scanning existing levels for an overlap. So the cost grows with the
square of the primitive count. Measured in milestone 299:

| primitives | `draw_calls` | result |
|---|---|---|
| 80 | 4.7 µs | 3 draw calls |
| 392 | 66 µs | 5 draw calls |
| 1302 | 597 µs | 5 draw calls |

Sixteen times the primitives, 127 times the cost. A dense screen — a long table, a
chart with many points — pays for it every frame.

### What to do

Whatever gets the overlap query below linear: a grid, an interval tree, sorting by one
axis and sweeping. The right answer probably depends on the fact that interfaces are
mostly axis-aligned rectangles that mostly do not overlap.

### Rules of the game

- The **plan must not change**. `crates/frus-gpu` has tests asserting the resulting
  draw-call counts, and `crates/frus-test`'s goldens will notice any ordering change.
- Bring numbers. `cargo bench -p frus-bench --bench batch` is the harness, and the
  table above is the baseline to beat.

### Where

`crates/frus-gpu/src/batch.rs`, `plan()` around line 157.
