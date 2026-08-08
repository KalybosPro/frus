# Jalon 35 — Layout: grid (`Grid`)

The dedicated layout milestone: an equal-column **grid**, through **taffy's CSS
Grid** (already in the dependency) rather than an improvised composite.

## Approach

`Grid` was missing because of a trap: a "builder" composite cannot rebuild
`Box<dyn Widget>` children. The right answer is not a widget that rebuilds rows,
but **delegating the arrangement to the layout engine**.

- `Style` gains `grid_columns: Option<usize>`.
- `to_taffy`: if `Some(n)` → `display: Grid` + `grid_template_columns = n × 1fr`.
  The children place themselves **automatically** (auto-flow, row by row); the
  rows are sized to their content → the container's height follows on its own.
- So `Grid` is a **normal container**: `cell()` is just a child `push`, with no
  special branch in `build_ui` and no ownership problem.

## API

```rust
Grid::new(3).gap(10.0).width(360.0)
    .cell(a).cell(b).cell(c)   // [a b c] / [d …]
    .cell(d)
```

## Demo

The Settings "About" tab: a **3-column** grid of statistics tiles (Total /
Active / Done).

## Tests

- `cells_flow_into_rows_and_columns`: in a 2-column grid, `a`/`b` on the same row
  (same `y`, `b` to the right), `c` under `a` (same `x`, lower `y`), `d` aligned
  (`b`'s column, `c`'s row), and **equal columns** (`a.width == b.width`). Direct
  proof that the grid arranges correctly.
- 69 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- **Equal columns** (`1fr`) only; no variable column widths (`px` / `auto` /
  `minmax`) and no cell spans yet.
- No `Table` (headers / cell borders) — a candidate for the next batch of
  widgets, built on `Grid`.
